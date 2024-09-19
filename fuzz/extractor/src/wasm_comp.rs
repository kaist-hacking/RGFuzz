use anyhow::{Error, anyhow};
use wasm_ast::{Instruction, ValueType, Module, FunctionIndex, MemoryType, Limit, FunctionType, VariableInstruction, Expression, Function, Export, ExportDescription, Name, Global, GlobalType, Element, ReferenceType, ElementInitializer, TableType, Table, Data, ImportDescription, Import, ModuleBuilder, VectorShape, IntegerType, FloatType, NumberType, SignExtension, IShape, FShape};
use enum_iterator::all;

#[derive(Clone, Debug, PartialEq)]
pub struct InstructionType {
    pub param_types: Vec<ValueType>,
    pub ret_types: Vec<ValueType>,
    pub has_operand: bool,
}

pub fn get_instruction_type(module: &Module, funcidx: FunctionIndex, instr: &Instruction) 
    -> Result<Vec<InstructionType>, Error> {
    Ok(match instr {
        // numeric instructions are not value-polymorphic
        Instruction::Numeric(i) => vec![{
            match i {
                wasm_ast::NumericInstruction::I32Constant(_) => // i32.const
                    InstructionType { param_types: vec![], ret_types: vec![ValueType::I32], has_operand: true },
                wasm_ast::NumericInstruction::I64Constant(_) => // i64.const
                    InstructionType { param_types: vec![], ret_types: vec![ValueType::I64], has_operand: true },
                wasm_ast::NumericInstruction::F32Constant(_) => // f32.const
                    InstructionType { param_types: vec![], ret_types: vec![ValueType::F32], has_operand: true },
                wasm_ast::NumericInstruction::F64Constant(_) => // f64.const
                    InstructionType { param_types: vec![], ret_types: vec![ValueType::F64], has_operand: true },
                wasm_ast::NumericInstruction::CountLeadingZeros(itype) | // inn.clz
                wasm_ast::NumericInstruction::CountTrailingZeros(itype) | // inn.ctz
                wasm_ast::NumericInstruction::CountOnes(itype) => // inn.popcnt
                    InstructionType { 
                        param_types: vec![ValueType::from(itype.clone())], 
                        ret_types: vec![ValueType::from(itype.clone())],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::AbsoluteValue(ftype) | // fnn.abs
                wasm_ast::NumericInstruction::Negate(ftype) | // fnn.neg
                wasm_ast::NumericInstruction::SquareRoot(ftype) | // fnn.sqrt
                wasm_ast::NumericInstruction::Ceiling(ftype) | // fnn.ceil
                wasm_ast::NumericInstruction::Floor(ftype) | // fnn.floor
                wasm_ast::NumericInstruction::Truncate(ftype) | // fnn.trunc
                wasm_ast::NumericInstruction::Nearest(ftype) => // fnn.nearest
                    InstructionType { 
                        param_types: vec![ValueType::from(ftype.clone())], 
                        ret_types: vec![ValueType::from(ftype.clone())],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::Add(ntype) | // xnn.add
                wasm_ast::NumericInstruction::Subtract(ntype) | // xnn.sub
                wasm_ast::NumericInstruction::Multiply(ntype) => // xnn.mul
                    InstructionType { 
                        param_types: vec![ValueType::from(ntype.clone()), ValueType::from(ntype.clone())], 
                        ret_types: vec![ValueType::from(ntype.clone())],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::DivideInteger(itype, _) | // inn.div_sx
                wasm_ast::NumericInstruction::Remainder(itype, _) | // inn.rem_sx
                wasm_ast::NumericInstruction::And(itype) | // inn.and
                wasm_ast::NumericInstruction::Or(itype) | // inn.or
                wasm_ast::NumericInstruction::Xor(itype) | // inn.xor
                wasm_ast::NumericInstruction::ShiftLeft(itype) | // inn.shl
                wasm_ast::NumericInstruction::ShiftRight(itype, _) | // inn.shr_sx
                wasm_ast::NumericInstruction::RotateLeft(itype) | // inn.rotl
                wasm_ast::NumericInstruction::RotateRight(itype) => // inn.rotr
                    InstructionType { 
                        param_types: vec![ValueType::from(itype.clone()), ValueType::from(itype.clone())], 
                        ret_types: vec![ValueType::from(itype.clone())],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::DivideFloat(ftype) | // fnn.div
                wasm_ast::NumericInstruction::Minimum(ftype) | // fnn.min
                wasm_ast::NumericInstruction::Maximum(ftype) | // fnn.max
                wasm_ast::NumericInstruction::CopySign(ftype) => // fnn.copysign
                    InstructionType { 
                        param_types: vec![ValueType::from(ftype.clone()), ValueType::from(ftype.clone())], 
                        ret_types: vec![ValueType::from(ftype.clone())],
                        has_operand: false,
                    },
                
                wasm_ast::NumericInstruction::EqualToZero(itype) => // inn.eqz
                    InstructionType { 
                        param_types: vec![ValueType::from(itype.clone())], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::Equal(ntype) | // xnn.eq
                wasm_ast::NumericInstruction::NotEqual(ntype) => // xnn.ne
                    InstructionType { 
                        param_types: vec![ValueType::from(ntype.clone()), ValueType::from(ntype.clone())], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::LessThanInteger(itype, _) | // inn.lt_sx
                wasm_ast::NumericInstruction::GreaterThanInteger(itype, _) | // inn.gt_sx
                wasm_ast::NumericInstruction::LessThanOrEqualToInteger(itype, _) | // inn.le_sx
                wasm_ast::NumericInstruction::GreaterThanOrEqualToInteger(itype, _) => // inn.ge_sx
                    InstructionType { 
                        param_types: vec![ValueType::from(itype.clone()), ValueType::from(itype.clone())], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::LessThanFloat(ftype) | // fnn.lt
                wasm_ast::NumericInstruction::GreaterThanFloat(ftype) | // fnn.gt
                wasm_ast::NumericInstruction::LessThanOrEqualToFloat(ftype) | // fnn.le
                wasm_ast::NumericInstruction::GreaterThanOrEqualToFloat(ftype) => // fnn.ge
                    InstructionType { 
                        param_types: vec![ValueType::from(ftype.clone()), ValueType::from(ftype.clone())], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::ExtendSigned8(itype) | // inn.extend8_s
                wasm_ast::NumericInstruction::ExtendSigned16(itype) => // inn.extend16_s
                    InstructionType { 
                        param_types: vec![ValueType::from(itype.clone())], 
                        ret_types: vec![ValueType::from(itype.clone())],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::ExtendSigned32 => // i64.extend32_s
                    InstructionType { 
                        param_types: vec![ValueType::I64], 
                        ret_types: vec![ValueType::I64],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::Wrap => // i32.wrap_i64
                    InstructionType { 
                        param_types: vec![ValueType::I64], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::ExtendWithSignExtension(_) => // i64.extend_i32_sx
                    InstructionType { 
                        param_types: vec![ValueType::I32], 
                        ret_types: vec![ValueType::I64],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::ConvertAndTruncate(itype, ftype, _) | // inn.trunc_fmm_sx
                wasm_ast::NumericInstruction::ConvertAndTruncateWithSaturation(itype, ftype, _) => // inn.trunc_sat_fmm_sx
                    InstructionType { 
                        param_types: vec![ValueType::from(ftype.clone())], 
                        ret_types: vec![ValueType::from(itype.clone())],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::Demote => // f32.demote_f64
                    InstructionType { 
                        param_types: vec![ValueType::F64], 
                        ret_types: vec![ValueType::F32],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::Promote => // f64.promote_f32
                    InstructionType { 
                        param_types: vec![ValueType::F32], 
                        ret_types: vec![ValueType::F64],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::Convert(ftype, itype, _) => // fnn.convert_imm_sx
                    InstructionType { 
                        param_types: vec![ValueType::from(itype.clone())], 
                        ret_types: vec![ValueType::from(ftype.clone())],
                        has_operand: false,
                    },
                wasm_ast::NumericInstruction::ReinterpretFloat(itype) => // inn.reinterpret_fmm
                    match itype {
                        wasm_ast::IntegerType::I32 => 
                            InstructionType { 
                                param_types: vec![ValueType::F32], 
                                ret_types: vec![ValueType::I32],
                                has_operand: false,
                            },
                        wasm_ast::IntegerType::I64 => 
                            InstructionType { 
                                param_types: vec![ValueType::F64], 
                                ret_types: vec![ValueType::I64],
                                has_operand: false,
                            },
                    }
                wasm_ast::NumericInstruction::ReinterpretInteger(ftype) => // fmm.reinterpret_imm
                    match ftype {
                        wasm_ast::FloatType::F32 => 
                            InstructionType { 
                                param_types: vec![ValueType::I32], 
                                ret_types: vec![ValueType::F32],
                                has_operand: false,
                            },
                        wasm_ast::FloatType::F64 => 
                            InstructionType { 
                                param_types: vec![ValueType::I64], 
                                ret_types: vec![ValueType::F64],
                                has_operand: false,
                            },
                    }
            }
        }],
        Instruction::Reference(i) => {
            match i {
                wasm_ast::ReferenceInstruction::Null(rtype) => vec![
                    InstructionType { 
                        param_types: vec![], 
                        ret_types: vec![ValueType::from(rtype.clone())],
                        has_operand: false, // actually true, but encoded with type
                    },
                ],
                wasm_ast::ReferenceInstruction::IsNull => vec![
                    InstructionType { 
                        param_types: vec![ValueType::FunctionReference], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                    InstructionType { 
                        param_types: vec![ValueType::ExternalReference], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                ],
                wasm_ast::ReferenceInstruction::Function(_) => vec![
                    InstructionType { 
                        param_types: vec![], 
                        ret_types: vec![ValueType::FunctionReference],
                        has_operand: true,
                    },
                ],
            }
        },
        Instruction::Parametric(i) => {
            match i {
                wasm_ast::ParametricInstruction::Drop => {
                    let valtypes = [
                        ValueType::I32, ValueType::I64, 
                        ValueType::F32, ValueType::F64,
                        ValueType::V128,
                    ];
                    let mut itype_vec = Vec::new();
                    for ty in valtypes {
                        itype_vec.push(InstructionType {
                            param_types: vec![ty.clone()],
                            ret_types: vec![],
                            has_operand: false,
                        });
                    }
                    itype_vec
                }
                wasm_ast::ParametricInstruction::Select(None) => {
                    let valtypes = [
                        ValueType::I32, ValueType::I64, 
                        ValueType::F32, ValueType::F64,
                        ValueType::V128,
                    ];
                    let mut itype_vec = Vec::new();
                    for ty in valtypes {
                        itype_vec.push(InstructionType {
                            param_types: vec![ty.clone(), ty.clone(), ValueType::I32],
                            ret_types: vec![ty.clone()],
                            has_operand: false,
                        });
                    }
                    itype_vec
                }
                wasm_ast::ParametricInstruction::Select(Some(types)) => {
                    assert!(types.len() == 1);
                    let ty = types[0];
                    vec![InstructionType {
                        param_types: vec![ty.clone(), ty.clone(), ValueType::I32],
                        ret_types: vec![ty.clone()],
                        has_operand: false, // actually true, but encoded with type
                    }]
                }
            }
        },
        Instruction::Variable(i) => {
            match i {
                wasm_ast::VariableInstruction::LocalGet(lidx) => {
                    let func = &module.functions().unwrap()[funcidx as usize];
                    let local_ty = func.locals().kinds()[*lidx as usize];
                    vec![InstructionType {
                        param_types: vec![],
                        ret_types: vec![local_ty.clone()],
                        has_operand: true,
                    }]
                },
                wasm_ast::VariableInstruction::LocalSet(lidx) => {
                    let func = &module.functions().unwrap()[funcidx as usize];
                    let local_ty = func.locals().kinds()[*lidx as usize];
                    vec![InstructionType {
                        param_types: vec![local_ty.clone()],
                        ret_types: vec![],
                        has_operand: true,
                    }]
                },
                wasm_ast::VariableInstruction::LocalTee(lidx) => {
                    let func = &module.functions().unwrap()[funcidx as usize];
                    let local_ty = func.locals().kinds()[*lidx as usize];
                    vec![InstructionType {
                        param_types: vec![local_ty.clone()],
                        ret_types: vec![local_ty.clone()],
                        has_operand: true,
                    }]
                },
                wasm_ast::VariableInstruction::GlobalGet(gidx) => {
                    let globals = module.globals().unwrap();
                    let global_ty = globals[*gidx as usize].kind().kind();
                    vec![InstructionType {
                        param_types: vec![],
                        ret_types: vec![global_ty.clone()],
                        has_operand: true,
                    }]
                },
                wasm_ast::VariableInstruction::GlobalSet(gidx) => {
                    let globals = module.globals().unwrap();
                    let global_ty = globals[*gidx as usize].kind().kind();
                    vec![InstructionType {
                        param_types: vec![global_ty.clone()],
                        ret_types: vec![],
                        has_operand: true,
                    }]
                },
            }
        },
        Instruction::Table(i) => vec![{
            match i {
                wasm_ast::TableInstruction::Get(tidx) => {
                    let tables = module.tables().unwrap();
                    let table_ty = tables[*tidx as usize].kind().kind();
                    InstructionType {
                        param_types: vec![ValueType::I32],
                        ret_types: vec![ValueType::from(table_ty)],
                        has_operand: true,
                    }
                },
                wasm_ast::TableInstruction::Set(tidx) => {
                    let tables = module.tables().unwrap();
                    let table_ty = tables[*tidx as usize].kind().kind();
                    InstructionType {
                        param_types: vec![ValueType::I32, ValueType::from(table_ty)],
                        ret_types: vec![],
                        has_operand: true,
                    }
                },
                wasm_ast::TableInstruction::Size(_) => 
                    InstructionType { 
                        param_types: vec![], 
                        ret_types: vec![ValueType::I32] ,
                        has_operand: true,
                    },
                wasm_ast::TableInstruction::Grow(tidx) => {
                    let tables = module.tables().unwrap();
                    let table_ty = tables[*tidx as usize].kind().kind();
                    InstructionType {
                        param_types: vec![ValueType::from(table_ty), ValueType::I32],
                        ret_types: vec![ValueType::I32],
                        has_operand: true,
                    }
                },
                wasm_ast::TableInstruction::Fill(tidx) => {
                    let tables = module.tables().unwrap();
                    let table_ty = tables[*tidx as usize].kind().kind();
                    InstructionType {
                        param_types: vec![ValueType::I32, ValueType::from(table_ty), ValueType::I32],
                        ret_types: vec![],
                        has_operand: true,
                    }
                },
                wasm_ast::TableInstruction::Copy(_, _) |
                wasm_ast::TableInstruction::Init(_, _) => 
                    InstructionType { 
                        param_types: vec![ValueType::I32, ValueType::I32, ValueType::I32], 
                        ret_types: vec![],
                        has_operand: true,
                    },
                wasm_ast::TableInstruction::ElementDrop(_) => 
                    InstructionType { param_types: vec![], ret_types: vec![], has_operand: true },
            }
        }],
        Instruction::Memory(i) => vec![{
            match i {
                wasm_ast::MemoryInstruction::Load(ntype, _) => 
                    InstructionType { 
                        param_types: vec![ValueType::I32], 
                        ret_types: vec![ValueType::from(ntype.clone())],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::Store(ntype, _) => 
                    InstructionType { 
                        param_types: vec![ValueType::I32, ValueType::from(ntype.clone())], 
                        ret_types: vec![],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::V128Load(_) =>
                    InstructionType {
                        param_types: vec![ValueType::I32],
                        ret_types: vec![ValueType::V128],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::V128Store(_) =>
                    InstructionType {
                        param_types: vec![ValueType::I32, ValueType::V128],
                        ret_types: vec![],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::Load8(itype, _, _) |
                wasm_ast::MemoryInstruction::Load16(itype, _, _) =>
                    InstructionType { 
                        param_types: vec![ValueType::I32], 
                        ret_types: vec![ValueType::from(itype.clone())],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::Load32(_, _) => 
                    InstructionType { 
                        param_types: vec![ValueType::I32], 
                        ret_types: vec![ValueType::I64],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::Store8(itype, _) |
                wasm_ast::MemoryInstruction::Store16(itype, _) => 
                    InstructionType { 
                        param_types: vec![ValueType::I32, ValueType::from(itype.clone())], 
                        ret_types: vec![],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::Store32(_) => 
                    InstructionType { 
                        param_types: vec![ValueType::I32, ValueType::I64], 
                        ret_types: vec![],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::V128Load8X8(..) |
                wasm_ast::MemoryInstruction::V128Load16X4(..) |
                wasm_ast::MemoryInstruction::V128Load32X2(..) |
                wasm_ast::MemoryInstruction::V128Load8Splat(..) |
                wasm_ast::MemoryInstruction::V128Load16Splat(..) |
                wasm_ast::MemoryInstruction::V128Load32Splat(..) |
                wasm_ast::MemoryInstruction::V128Load64Splat(..) |
                wasm_ast::MemoryInstruction::V128Load32Zero(..) |
                wasm_ast::MemoryInstruction::V128Load64Zero(..) =>
                    InstructionType {
                        param_types: vec![ValueType::I32],
                        ret_types: vec![ValueType::V128],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::V128Load8Lane(..) |
                wasm_ast::MemoryInstruction::V128Load16Lane(..) |
                wasm_ast::MemoryInstruction::V128Load32Lane(..) |
                wasm_ast::MemoryInstruction::V128Load64Lane(..) |
                wasm_ast::MemoryInstruction::V128Store8Lane(..) |
                wasm_ast::MemoryInstruction::V128Store16Lane(..) |
                wasm_ast::MemoryInstruction::V128Store32Lane(..) |
                wasm_ast::MemoryInstruction::V128Store64Lane(..) =>
                    InstructionType {
                        param_types: vec![ValueType::I32, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: true,
                    },
                wasm_ast::MemoryInstruction::Size => 
                    InstructionType { 
                        param_types: vec![], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::MemoryInstruction::Grow => 
                    InstructionType { 
                        param_types: vec![ValueType::I32], 
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::MemoryInstruction::Fill |
                wasm_ast::MemoryInstruction::Copy => 
                    InstructionType { 
                        param_types: vec![ValueType::I32, ValueType::I32, ValueType::I32], 
                        ret_types: vec![],
                        has_operand: false,
                    },
                wasm_ast::MemoryInstruction::Init(_) => 
                    InstructionType { 
                        param_types: vec![ValueType::I32, ValueType::I32, ValueType::I32], 
                        ret_types: vec![],
                        has_operand: true,
                    }, 
                wasm_ast::MemoryInstruction::DataDrop(_) => 
                    InstructionType { param_types: vec![], ret_types: vec![], has_operand: true },
            }
        }],
        Instruction::Control(i) => {
            match i {
                wasm_ast::ControlInstruction::Nop => 
                    vec![InstructionType { param_types: vec![], ret_types: vec![], has_operand: false }],
                wasm_ast::ControlInstruction::Unreachable => 
                    return Err(anyhow!("Stack-polymorphic instructions not supported")),
                wasm_ast::ControlInstruction::Block(btype, _) |
                wasm_ast::ControlInstruction::Loop(btype, _) => {
                    // has_operand is false since types are encoded
                    match btype {
                        wasm_ast::BlockType::None => vec![InstructionType { param_types: vec![], ret_types: vec![], has_operand: false }],
                        wasm_ast::BlockType::Index(tidx) => {
                            let func_type = module.function_types().unwrap();
                            let ty = &func_type[*tidx as usize];
                            let params = ty.parameters().kinds().to_vec();
                            let results = ty.results().kinds().to_vec();
                            vec![InstructionType { param_types: params, ret_types: results, has_operand: false }]
                        },
                        wasm_ast::BlockType::ValueType(ty) => 
                            vec![InstructionType { param_types: vec![], ret_types: vec![ty.clone()], has_operand: false }],
                    }
                }
                wasm_ast::ControlInstruction::If(btype, _, _) => {
                    match btype {
                        wasm_ast::BlockType::None => vec![InstructionType { param_types: vec![ValueType::I32], ret_types: vec![], has_operand: false }],
                        wasm_ast::BlockType::Index(tidx) => {
                            let func_type = module.function_types().unwrap();
                            let ty = &func_type[*tidx as usize];
                            let mut params = ty.parameters().kinds().to_vec();
                            params.push(ValueType::I32);
                            let results = ty.results().kinds().to_vec();
                            vec![InstructionType { param_types: params, ret_types: results, has_operand: true }] // not sure about this
                        },
                        wasm_ast::BlockType::ValueType(ty) => 
                            vec![InstructionType { param_types: vec![ValueType::I32], ret_types: vec![ty.clone()], has_operand: false }],
                    }
                },
                wasm_ast::ControlInstruction::Branch(_) |
                wasm_ast::ControlInstruction::BranchIf(_) |
                wasm_ast::ControlInstruction::BranchTable(_, _) |
                wasm_ast::ControlInstruction::Return => 
                    return Err(anyhow!("Stack-polymorphic instructions not supported")),
                wasm_ast::ControlInstruction::Call(fidx) => {
                    let func = module.functions().unwrap();
                    let func_ty_idx = func[*fidx as usize].kind();
                    let func_type = module.function_types().unwrap();
                    let func_ty = &func_type[func_ty_idx as usize];
                    let params = func_ty.parameters().kinds().to_vec();
                    let results = func_ty.results().kinds().to_vec();
                    vec![InstructionType { param_types: params, ret_types: results, has_operand: true }] // not sure about this
                },
                wasm_ast::ControlInstruction::CallIndirect(type_idx, _) => {
                    let func_type = module.function_types().unwrap();
                    let func_ty = &func_type[*type_idx as usize];
                    let mut params = func_ty.parameters().kinds().to_vec();
                    params.push(ValueType::I32);
                    let results = func_ty.results().kinds().to_vec();
                    vec![InstructionType { param_types: params, ret_types: results, has_operand: true }] // not sure about this
                },
            }
        },
        Instruction::Vector(i) => {
            fn unpack_shape(shape: &VectorShape) -> ValueType {
                match shape {
                    VectorShape::I8X16 => ValueType::I32,
                    VectorShape::I16X8 => ValueType::I32,
                    VectorShape::I32X4 => ValueType::I32,
                    VectorShape::I64X2 => ValueType::I64,
                    VectorShape::F32X4 => ValueType::F32,
                    VectorShape::F64X2 => ValueType::F64,
                }
            }
            let instr_type = match i {
                wasm_ast::VectorInstruction::V128Constant(_) => 
                    InstructionType {
                        param_types: vec![],
                        ret_types: vec![ValueType::V128],
                        has_operand: true,
                    },
                wasm_ast::VectorInstruction::Not => 
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::And |
                wasm_ast::VectorInstruction::AndNot |
                wasm_ast::VectorInstruction::Or |
                wasm_ast::VectorInstruction::Xor => 
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::BitSelect => 
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::AnyTrue => 
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::I8X16Shuffle(_) => 
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: true,
                    },
                wasm_ast::VectorInstruction::I8X16Swizzle => 
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::Splat(shape) => 
                    InstructionType {
                        param_types: vec![unpack_shape(shape)],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::I8X16ExtractLane(_, _) => 
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![unpack_shape(&VectorShape::I8X16)],
                        has_operand: true,
                    },
                wasm_ast::VectorInstruction::I16X8ExtractLane(_, _) => 
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![unpack_shape(&VectorShape::I16X8)],
                        has_operand: true,
                    },
                wasm_ast::VectorInstruction::I32X4ExtractLane(_) => 
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![unpack_shape(&VectorShape::I32X4)],
                        has_operand: true,
                    },
                wasm_ast::VectorInstruction::I64X2ExtractLane(_) => 
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![unpack_shape(&VectorShape::I64X2)],
                        has_operand: true,
                    },
                wasm_ast::VectorInstruction::FExtractLane(shape, _) => 
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![unpack_shape(&VectorShape::from(shape.clone()))],
                        has_operand: true,
                    },
                wasm_ast::VectorInstruction::ReplaceLane(shape, _) =>
                    InstructionType {
                        param_types: vec![ValueType::V128, unpack_shape(shape)],
                        ret_types: vec![ValueType::V128],
                        has_operand: true,
                    },
                wasm_ast::VectorInstruction::IAbs(_) |
                wasm_ast::VectorInstruction::INeg(_) |
                wasm_ast::VectorInstruction::FAbs(_) |
                wasm_ast::VectorInstruction::FNeg(_) |
                wasm_ast::VectorInstruction::Sqrt(_) |
                wasm_ast::VectorInstruction::Ceil(_) |
                wasm_ast::VectorInstruction::Floor(_) |
                wasm_ast::VectorInstruction::Trunc(_) |
                wasm_ast::VectorInstruction::Nearest(_) |
                wasm_ast::VectorInstruction::I8X16Popcnt =>
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::IAdd(_) |
                wasm_ast::VectorInstruction::ISub(_) |
                wasm_ast::VectorInstruction::FAdd(_) |
                wasm_ast::VectorInstruction::FSub(_) |
                wasm_ast::VectorInstruction::FMul(_) |
                wasm_ast::VectorInstruction::FDiv(_) |
                wasm_ast::VectorInstruction::FMin(_) |
                wasm_ast::VectorInstruction::FMax(_) |
                wasm_ast::VectorInstruction::Pmin(_) |
                wasm_ast::VectorInstruction::Pmax(_) |
                wasm_ast::VectorInstruction::I8X16Min(_) |
                wasm_ast::VectorInstruction::I8X16Max(_) |
                wasm_ast::VectorInstruction::I16X8Min(_) |
                wasm_ast::VectorInstruction::I16X8Max(_) |
                wasm_ast::VectorInstruction::I32X4Min(_) |
                wasm_ast::VectorInstruction::I32X4Max(_) |
                wasm_ast::VectorInstruction::I8X16AddSat(_) |
                wasm_ast::VectorInstruction::I8X16SubSat(_) |
                wasm_ast::VectorInstruction::I16X8AddSat(_) |
                wasm_ast::VectorInstruction::I16X8SubSat(_) |
                wasm_ast::VectorInstruction::I16X8Mul |
                wasm_ast::VectorInstruction::I32X4Mul |
                wasm_ast::VectorInstruction::I64X2Mul |
                wasm_ast::VectorInstruction::I8X16Avgr |
                wasm_ast::VectorInstruction::I16X8Avgr |
                wasm_ast::VectorInstruction::I16X8Q15MulrSat =>
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::I8X16Eq |
                wasm_ast::VectorInstruction::I8X16Ne |
                wasm_ast::VectorInstruction::I8X16Lt(_) |
                wasm_ast::VectorInstruction::I8X16Gt(_) |
                wasm_ast::VectorInstruction::I8X16Le(_) |
                wasm_ast::VectorInstruction::I8X16Ge(_) |
                wasm_ast::VectorInstruction::I16X8Eq |
                wasm_ast::VectorInstruction::I16X8Ne |
                wasm_ast::VectorInstruction::I16X8Lt(_) |
                wasm_ast::VectorInstruction::I16X8Gt(_) |
                wasm_ast::VectorInstruction::I16X8Le(_) |
                wasm_ast::VectorInstruction::I16X8Ge(_) |
                wasm_ast::VectorInstruction::I32X4Eq |
                wasm_ast::VectorInstruction::I32X4Ne |
                wasm_ast::VectorInstruction::I32X4Lt(_) |
                wasm_ast::VectorInstruction::I32X4Gt(_) |
                wasm_ast::VectorInstruction::I32X4Le(_) |
                wasm_ast::VectorInstruction::I32X4Ge(_) |
                wasm_ast::VectorInstruction::I64X2Eq |
                wasm_ast::VectorInstruction::I64X2Ne |
                wasm_ast::VectorInstruction::I64X2Lt |
                wasm_ast::VectorInstruction::I64X2Gt |
                wasm_ast::VectorInstruction::I64X2Le |
                wasm_ast::VectorInstruction::I64X2Ge |
                wasm_ast::VectorInstruction::FEq(_) |
                wasm_ast::VectorInstruction::FNe(_) |
                wasm_ast::VectorInstruction::FLt(_) |
                wasm_ast::VectorInstruction::FGt(_) |
                wasm_ast::VectorInstruction::FLe(_) |
                wasm_ast::VectorInstruction::FGe(_) =>
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::Shl(_) |
                wasm_ast::VectorInstruction::Shr(_, _) =>
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::I32],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::AllTrue(_) =>
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::I16X8ExtendLowI8X16(_) |
                wasm_ast::VectorInstruction::I16X8ExtendHighI8X16(_) |
                wasm_ast::VectorInstruction::I32X4ExtendLowI16X8(_) |
                wasm_ast::VectorInstruction::I32X4ExtendHighI16X8(_) |
                wasm_ast::VectorInstruction::I64X2ExtendLowI32X4(_) |
                wasm_ast::VectorInstruction::I64X2ExtendHighI32X4(_) |
                wasm_ast::VectorInstruction::I32X4TruncSatF32X4(_) |
                wasm_ast::VectorInstruction::I32X4TruncSatF64X2Zero(_) |
                wasm_ast::VectorInstruction::F32X4ConvertI32X4(_) |
                wasm_ast::VectorInstruction::F32X4DemoteF64X2Zero |
                wasm_ast::VectorInstruction::F64X2ConvertLowI32X4(_) |
                wasm_ast::VectorInstruction::F64X2PromoteLowF32X4 =>
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::I8X16NarrowI16X8(_) |
                wasm_ast::VectorInstruction::I16X8NarrowI32X4(_) =>
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::Bitmask(_) =>
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![ValueType::I32],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::I32X4DotI16X8 =>
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::I16X8ExtmulLowI8X16(_) |
                wasm_ast::VectorInstruction::I16X8ExtmulHighI8X16(_) |
                wasm_ast::VectorInstruction::I32X4ExtmulLowI16X8(_) |
                wasm_ast::VectorInstruction::I32X4ExtmulHighI16X8(_) |
                wasm_ast::VectorInstruction::I64X2ExtmulLowI32X4(_) |
                wasm_ast::VectorInstruction::I64X2ExtmulHighI32X4(_) =>
                    InstructionType {
                        param_types: vec![ValueType::V128, ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
                wasm_ast::VectorInstruction::I16X8ExtaddPairwiseI8X16(_) |
                wasm_ast::VectorInstruction::I32X4ExtaddPairwiseI16X8(_) =>
                    InstructionType {
                        param_types: vec![ValueType::V128],
                        ret_types: vec![ValueType::V128],
                        has_operand: false,
                    },
            };

            vec![instr_type]
        },
    })
}

pub fn get_instruction_name(instr: &Instruction) -> String {
    fn integertype_to_name(ty: &IntegerType) -> &str {
        match ty {
            IntegerType::I32 => "i32",
            IntegerType::I64 => "i64",
        }
    }
    fn floattype_to_name(ty: &FloatType) -> &str {
        match ty {
            FloatType::F32 => "f32",
            FloatType::F64 => "f64",
        }
    }
    fn numbertype_to_name(ty: &NumberType) -> &str {
        match ty {
            NumberType::I32 => "i32",
            NumberType::I64 => "i64",
            NumberType::F32 => "f32",
            NumberType::F64 => "f64",
        }
    }
    fn signextension_to_name(se: &SignExtension) -> &str {
        match se {
            SignExtension::Signed => "s",
            SignExtension::Unsigned => "u",
        }
    }
    fn vshape_to_name(v: &VectorShape) -> &str {
        match v {
            VectorShape::I8X16 => "i8x16",
            VectorShape::I16X8 => "i16x8",
            VectorShape::I32X4 => "i32x4",
            VectorShape::I64X2 => "i64x2",
            VectorShape::F32X4 => "f32x4",
            VectorShape::F64X2 => "f64x2",
        }
    }
    fn ishape_to_name(i: &IShape) -> &str {
        match i {
            IShape::I8X16 => "i8x16",
            IShape::I16X8 => "i16x8",
            IShape::I32X4 => "i32x4",
            IShape::I64X2 => "i64x2",
        }
    }
    fn fshape_to_name(f: &FShape) -> &str {
        match f {
            FShape::F32X4 => "f32x4",
            FShape::F64X2 => "f64x2",
        }
    }
    let name_str = match instr {
        Instruction::Numeric(i) => {
            match i {
                wasm_ast::NumericInstruction::I32Constant(_) => "i32.const",
                wasm_ast::NumericInstruction::I64Constant(_) => "i64.const",
                wasm_ast::NumericInstruction::F32Constant(_) => "f32.const",
                wasm_ast::NumericInstruction::F64Constant(_) => "f64.const",
                wasm_ast::NumericInstruction::CountLeadingZeros(ty) => 
                    return format!("{}.clz", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::CountTrailingZeros(ty) => 
                    return format!("{}.ctz", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::CountOnes(ty) => 
                    return format!("{}.popcnt", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::AbsoluteValue(ty) => 
                    return format!("{}.abs", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::Negate(ty) => 
                    return format!("{}.neg", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::SquareRoot(ty) => 
                    return format!("{}.sqrt", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::Ceiling(ty) => 
                    return format!("{}.ceil", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::Floor(ty) => 
                    return format!("{}.floor", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::Truncate(ty) => 
                    return format!("{}.trunc", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::Nearest(ty) => 
                    return format!("{}.nearest", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::Add(ty) => 
                    return format!("{}.add", numbertype_to_name(ty)),
                wasm_ast::NumericInstruction::Subtract(ty) => 
                    return format!("{}.sub", numbertype_to_name(ty)),
                wasm_ast::NumericInstruction::Multiply(ty) => 
                    return format!("{}.mul", numbertype_to_name(ty)),
                wasm_ast::NumericInstruction::DivideInteger(ty, se) => 
                    return format!("{}.div_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::NumericInstruction::DivideFloat(ty) => 
                    return format!("{}.div", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::Remainder(ty, se) => 
                    return format!("{}.rem_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::NumericInstruction::And(ty) => 
                    return format!("{}.and", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::Or(ty) => 
                    return format!("{}.or", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::Xor(ty) => 
                    return format!("{}.xor", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::ShiftLeft(ty) => 
                    return format!("{}.shl", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::ShiftRight(ty, se) => 
                    return format!("{}.shr_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::NumericInstruction::RotateLeft(ty) => 
                    return format!("{}.rotl", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::RotateRight(ty) => 
                    return format!("{}.rotr", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::Minimum(ty) => 
                    return format!("{}.min", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::Maximum(ty) => 
                    return format!("{}.max", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::CopySign(ty) => 
                    return format!("{}.copysign", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::EqualToZero(ty) =>
                    return format!("{}.eqz", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::Equal(ty) => 
                    return format!("{}.eq", numbertype_to_name(ty)),
                wasm_ast::NumericInstruction::NotEqual(ty) => 
                    return format!("{}.ne", numbertype_to_name(ty)),
                wasm_ast::NumericInstruction::LessThanInteger(ty, se) => 
                    return format!("{}.lt_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::NumericInstruction::LessThanFloat(ty) => 
                    return format!("{}.lt", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::GreaterThanInteger(ty, se) => 
                    return format!("{}.gt_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::NumericInstruction::GreaterThanFloat(ty) => 
                    return format!("{}.gt", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::LessThanOrEqualToInteger(ty, se) => 
                    return format!("{}.le_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::NumericInstruction::LessThanOrEqualToFloat(ty) => 
                    return format!("{}.le", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::GreaterThanOrEqualToInteger(ty, se) => 
                    return format!("{}.ge_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::NumericInstruction::GreaterThanOrEqualToFloat(ty) => 
                    return format!("{}.ge", floattype_to_name(ty)),
                wasm_ast::NumericInstruction::ExtendSigned8(ty) => 
                    return format!("{}.extend8_s", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::ExtendSigned16(ty) => 
                    return format!("{}.extend16_s", integertype_to_name(ty)),
                wasm_ast::NumericInstruction::ExtendSigned32 => "i64.extend32_s",
                wasm_ast::NumericInstruction::Wrap => "i32.wrap_i64",
                wasm_ast::NumericInstruction::ExtendWithSignExtension(se) => 
                    return format!("i64.extend_i32_{}", signextension_to_name(se)),
                wasm_ast::NumericInstruction::ConvertAndTruncate(ity, fty, se) => 
                    return format!(
                        "{}.trunc_{}_{}", 
                        integertype_to_name(ity), 
                        floattype_to_name(fty), 
                        signextension_to_name(se)
                    ),
                wasm_ast::NumericInstruction::ConvertAndTruncateWithSaturation(ity, fty, se) => 
                    return format!(
                        "{}.trunc_sat_{}_{}", 
                        integertype_to_name(ity), 
                        floattype_to_name(fty), 
                        signextension_to_name(se)
                    ),
                wasm_ast::NumericInstruction::Demote => "f32.demote_f64",
                wasm_ast::NumericInstruction::Promote => "f64.promote_f32",
                wasm_ast::NumericInstruction::Convert(fty, ity, se) => 
                    return format!(
                        "{}.convert_{}_{}", 
                        floattype_to_name(fty), 
                        integertype_to_name(ity), 
                        signextension_to_name(se)
                    ),
                wasm_ast::NumericInstruction::ReinterpretFloat(IntegerType::I32) => "i32.reinterpret_f32",
                wasm_ast::NumericInstruction::ReinterpretFloat(IntegerType::I64) => "i64.reinterpret_f64",
                wasm_ast::NumericInstruction::ReinterpretInteger(FloatType::F32) => "f32.reinterpret_i32",
                wasm_ast::NumericInstruction::ReinterpretInteger(FloatType::F64) => "f64.reinterpret_i64",
            }
        },
        Instruction::Vector(i) => {
            match i {
                wasm_ast::VectorInstruction::V128Constant(_) => "v128.const",
                wasm_ast::VectorInstruction::Not => "v128.not",
                wasm_ast::VectorInstruction::And => "v128.and",
                wasm_ast::VectorInstruction::AndNot => "v128.andnot",
                wasm_ast::VectorInstruction::Or => "v128.or",
                wasm_ast::VectorInstruction::Xor => "v128.xor",
                wasm_ast::VectorInstruction::BitSelect => "v128.bitselect",
                wasm_ast::VectorInstruction::AnyTrue => "v128.any_true",
                wasm_ast::VectorInstruction::I8X16Shuffle(_) => "i8x16.shuffle",
                wasm_ast::VectorInstruction::I8X16Swizzle => "i8x16.swizzle",
                wasm_ast::VectorInstruction::Splat(v) => 
                    return format!("{}.splat", vshape_to_name(v)),
                wasm_ast::VectorInstruction::I8X16ExtractLane(se, _) => 
                    return format!("i8x16.extract_lane_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8ExtractLane(se, _) => 
                    return format!("i16x8.extract_lane_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4ExtractLane(_) => "i32x4.extract_lane",
                wasm_ast::VectorInstruction::I64X2ExtractLane(_) => "i64x2.extract_lane",
                wasm_ast::VectorInstruction::FExtractLane(f, _) => 
                    return format!("{}.extract_lane", fshape_to_name(f)),
                wasm_ast::VectorInstruction::ReplaceLane(v, _) => 
                    return format!("{}.replace_lane", vshape_to_name(v)),
                wasm_ast::VectorInstruction::I8X16Eq => "i8x16.eq",
                wasm_ast::VectorInstruction::I8X16Ne => "i8x16.ne",
                wasm_ast::VectorInstruction::I8X16Lt(se) => 
                    return format!("i8x16.lt_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I8X16Gt(se) => 
                    return format!("i8x16.gt_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I8X16Le(se) => 
                    return format!("i8x16.le_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I8X16Ge(se) => 
                    return format!("i8x16.ge_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8Eq => "i16x8.eq",
                wasm_ast::VectorInstruction::I16X8Ne => "i16x8.ne",
                wasm_ast::VectorInstruction::I16X8Lt(se) => 
                    return format!("i16x8.lt_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8Gt(se) => 
                    return format!("i16x8.gt_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8Le(se) => 
                    return format!("i16x8.le_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8Ge(se) => 
                    return format!("i16x8.ge_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4Eq => "i32x4.eq",
                wasm_ast::VectorInstruction::I32X4Ne => "i32x4.ne",
                wasm_ast::VectorInstruction::I32X4Lt(se) => 
                    return format!("i32x4.lt_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4Gt(se) => 
                    return format!("i32x4.le_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4Le(se) => 
                    return format!("i32x4.gt_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4Ge(se) => 
                    return format!("i32x4.ge_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I64X2Eq => "i64x2.eq",
                wasm_ast::VectorInstruction::I64X2Ne => "i64x2.ne",
                wasm_ast::VectorInstruction::I64X2Lt => "i64x2.lt_s",
                wasm_ast::VectorInstruction::I64X2Gt => "i64x2.gt_s",
                wasm_ast::VectorInstruction::I64X2Le => "i64x2.le_s",
                wasm_ast::VectorInstruction::I64X2Ge => "i64x2.ge_s",
                wasm_ast::VectorInstruction::FEq(f) => 
                    return format!("{}.eq", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FNe(f) => 
                    return format!("{}.ne", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FLt(f) => 
                    return format!("{}.lt", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FGt(f) => 
                    return format!("{}.gt", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FLe(f) => 
                    return format!("{}.le", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FGe(f) => 
                    return format!("{}.ge", fshape_to_name(f)),
                wasm_ast::VectorInstruction::IAbs(i) =>  
                    return format!("{}.abs", ishape_to_name(i)),
                wasm_ast::VectorInstruction::INeg(i) =>  
                    return format!("{}.neg", ishape_to_name(i)),
                wasm_ast::VectorInstruction::I8X16Popcnt => "i8x16.popcnt",
                wasm_ast::VectorInstruction::I16X8Q15MulrSat => "i16x8.q15mulr_sat_s",
                wasm_ast::VectorInstruction::I32X4DotI16X8 => "i32x4.dot_i16x8_s",
                wasm_ast::VectorInstruction::FAbs(f) => 
                    return format!("{}.abs", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FNeg(f) => 
                    return format!("{}.neg", fshape_to_name(f)),
                wasm_ast::VectorInstruction::Sqrt(f) => 
                    return format!("{}.sqrt", fshape_to_name(f)),
                wasm_ast::VectorInstruction::Ceil(f) => 
                    return format!("{}.ceil", fshape_to_name(f)),
                wasm_ast::VectorInstruction::Floor(f) => 
                    return format!("{}.floor", fshape_to_name(f)),
                wasm_ast::VectorInstruction::Trunc(f) => 
                    return format!("{}.trunc", fshape_to_name(f)),
                wasm_ast::VectorInstruction::Nearest(f) => 
                    return format!("{}.nearest", fshape_to_name(f)),
                wasm_ast::VectorInstruction::AllTrue(i) =>  
                    return format!("{}.all_true", ishape_to_name(i)),
                wasm_ast::VectorInstruction::Bitmask(i) =>  
                    return format!("{}.bitmask", ishape_to_name(i)),
                wasm_ast::VectorInstruction::I8X16NarrowI16X8(se) => 
                    return format!("i8x16.narrow_i16x8_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8NarrowI32X4(se) => 
                    return format!("i16x8.narrow_i32x4_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8ExtendLowI8X16(se) => 
                    return format!("i16x8.extend_low_i8x16_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8ExtendHighI8X16(se) => 
                    return format!("i16x8.extend_high_i8x16_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4ExtendLowI16X8(se) => 
                    return format!("i32x4.extend_low_i16x8_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4ExtendHighI16X8(se) => 
                    return format!("i32x4.extend_high_i16x8_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I64X2ExtendLowI32X4(se) =>
                    return format!("i64x2.extend_low_i32x4_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I64X2ExtendHighI32X4(se) =>
                    return format!("i64x2.extend_high_i32x4_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::Shl(i) =>  
                    return format!("{}.shl", ishape_to_name(i)),
                wasm_ast::VectorInstruction::Shr(i, se) =>  
                    return format!("{}.shr_{}", ishape_to_name(i), signextension_to_name(se)),
                wasm_ast::VectorInstruction::IAdd(i) =>  
                    return format!("{}.add", ishape_to_name(i)),
                wasm_ast::VectorInstruction::ISub(i) =>  
                    return format!("{}.sub", ishape_to_name(i)),
                wasm_ast::VectorInstruction::I8X16Min(se) => 
                    return format!("i8x16.min_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I8X16Max(se) => 
                    return format!("i8x16.max_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8Min(se) => 
                    return format!("i16x8.min_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8Max(se) => 
                    return format!("i16x8.max_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4Min(se) => 
                    return format!("i32x4.min_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4Max(se) => 
                    return format!("i32x4.max_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I8X16AddSat(se) => 
                    return format!("i8x16.add_sat_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I8X16SubSat(se) => 
                    return format!("i8x16.sub_sat_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8AddSat(se) => 
                    return format!("i16x8.add_sat_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8SubSat(se) => 
                    return format!("i16x8.sub_sat_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8Mul => "i16x8.mul",
                wasm_ast::VectorInstruction::I32X4Mul => "i32x4.mul",
                wasm_ast::VectorInstruction::I64X2Mul => "i64x2.mul",
                wasm_ast::VectorInstruction::I8X16Avgr => "i8x16.avgr_u",
                wasm_ast::VectorInstruction::I16X8Avgr => "i16x8.avgr_u",
                wasm_ast::VectorInstruction::I16X8ExtmulLowI8X16(se) => 
                    return format!("i16x8.extmul_low_i8x16_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8ExtmulHighI8X16(se) => 
                    return format!("i16x8.extmul_high_i8x16_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4ExtmulLowI16X8(se) => 
                    return format!("i32x4.extmul_low_i16x8_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4ExtmulHighI16X8(se) => 
                    return format!("i32x4.extmul_high_i16x8_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I64X2ExtmulLowI32X4(se) => 
                    return format!("i64x2.extmul_low_i32x4_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I64X2ExtmulHighI32X4(se) => 
                    return format!("i64x2.extmul_high_i32x4_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I16X8ExtaddPairwiseI8X16(se) => 
                    return format!("i16x8.extadd_pairwise_i8x16_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4ExtaddPairwiseI16X8(se) => 
                    return format!("i32x4.extadd_pairwise_i16x8_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::FAdd(f) => 
                    return format!("{}.add", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FSub(f) => 
                    return format!("{}.sub", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FMul(f) => 
                    return format!("{}.mul", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FDiv(f) => 
                    return format!("{}.div", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FMin(f) => 
                    return format!("{}.min", fshape_to_name(f)),
                wasm_ast::VectorInstruction::FMax(f) => 
                    return format!("{}.max", fshape_to_name(f)),
                wasm_ast::VectorInstruction::Pmin(f) => 
                    return format!("{}.pmin", fshape_to_name(f)),
                wasm_ast::VectorInstruction::Pmax(f) => 
                    return format!("{}.pmax", fshape_to_name(f)),
                wasm_ast::VectorInstruction::I32X4TruncSatF32X4(se) => 
                    return format!("i32x4.trunc_sat_f32x4_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::I32X4TruncSatF64X2Zero(se) => 
                    return format!("i32x4.trunc_sat_f64x2_{}_zero", signextension_to_name(se)),
                wasm_ast::VectorInstruction::F32X4ConvertI32X4(se) => 
                    return format!("f32x4.convert_i32x4_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::F32X4DemoteF64X2Zero => "f32x4.demote_f64x2_zero",
                wasm_ast::VectorInstruction::F64X2ConvertLowI32X4(se) => 
                    return format!("f64x2.convert_low_i32x4_{}", signextension_to_name(se)),
                wasm_ast::VectorInstruction::F64X2PromoteLowF32X4 => "f64x2.promote_low_f32x4",
            }
        },
        Instruction::Reference(i) => {
            match i {
                wasm_ast::ReferenceInstruction::Null(_) => "ref.null",
                wasm_ast::ReferenceInstruction::IsNull => "ref.is_null",
                wasm_ast::ReferenceInstruction::Function(_) => "ref.func",
            }
        },
        Instruction::Parametric(i) => {
            match i {
                wasm_ast::ParametricInstruction::Drop => "drop",
                wasm_ast::ParametricInstruction::Select(None) => "select",
                wasm_ast::ParametricInstruction::Select(Some(_)) => "select_t",
            }
        },
        Instruction::Variable(i) => {
            match i {
                VariableInstruction::LocalGet(_) => "local.get",
                VariableInstruction::LocalSet(_) => "local.set",
                VariableInstruction::LocalTee(_) => "local.tee",
                VariableInstruction::GlobalGet(_) => "global.get",
                VariableInstruction::GlobalSet(_) => "global.set",
            }
        },
        Instruction::Table(i) => {
            match i {
                wasm_ast::TableInstruction::Get(_) => "table.get",
                wasm_ast::TableInstruction::Set(_) => "table.set",
                wasm_ast::TableInstruction::Size(_) => "table.size",
                wasm_ast::TableInstruction::Grow(_) => "table.grow",
                wasm_ast::TableInstruction::Fill(_) => "table.fill",
                wasm_ast::TableInstruction::Copy(_, _) => "table.copy",
                wasm_ast::TableInstruction::Init(_, _) => "table.init",
                wasm_ast::TableInstruction::ElementDrop(_) => "elem.drop",
            }
        },
        Instruction::Memory(i) => {
            match i {
                wasm_ast::MemoryInstruction::Load(ty, _) => 
                    return format!("{}.load", numbertype_to_name(ty)),
                wasm_ast::MemoryInstruction::Store(ty, _) => 
                    return format!("{}.load", numbertype_to_name(ty)),
                wasm_ast::MemoryInstruction::V128Load(_) => "v128.load",
                wasm_ast::MemoryInstruction::V128Store(_) => "v128.store",
                wasm_ast::MemoryInstruction::Load8(ty, se, _) =>
                    return format!("{}.load8_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::MemoryInstruction::Load16(ty, se, _) =>
                    return format!("{}.load16_{}", integertype_to_name(ty), signextension_to_name(se)),
                wasm_ast::MemoryInstruction::Load32(se, _) =>
                    return format!("i64.load32_{}", signextension_to_name(se)),
                wasm_ast::MemoryInstruction::Store8(ty, _) =>
                    return format!("{}.store8", integertype_to_name(ty)),
                wasm_ast::MemoryInstruction::Store16(ty, _) =>
                    return format!("{}.store16", integertype_to_name(ty)),
                wasm_ast::MemoryInstruction::Store32(_) => "i64.store32",
                wasm_ast::MemoryInstruction::V128Load8X8(se, _) =>
                    return format!("v128.load8x8_{}", signextension_to_name(se)),
                wasm_ast::MemoryInstruction::V128Load16X4(se, _) =>
                    return format!("v128.load16x4_{}", signextension_to_name(se)),
                wasm_ast::MemoryInstruction::V128Load32X2(se, _) =>
                    return format!("v128.load32x2_{}", signextension_to_name(se)),
                wasm_ast::MemoryInstruction::V128Load32Zero(_) => "v128.load32_zero",
                wasm_ast::MemoryInstruction::V128Load64Zero(_) => "v128.load64_zero",
                wasm_ast::MemoryInstruction::V128Load8Splat(_) => "v128.load8_splat",
                wasm_ast::MemoryInstruction::V128Load16Splat(_) => "v128.load16_splat",
                wasm_ast::MemoryInstruction::V128Load32Splat(_) => "v128.load32_splat",
                wasm_ast::MemoryInstruction::V128Load64Splat(_) => "v128.load64_splat",
                wasm_ast::MemoryInstruction::V128Load8Lane(_, _) => "v128.load8_lane",
                wasm_ast::MemoryInstruction::V128Load16Lane(_, _) => "v128.load16_lane",
                wasm_ast::MemoryInstruction::V128Load32Lane(_, _) => "v128.load32_lane",
                wasm_ast::MemoryInstruction::V128Load64Lane(_, _) => "v128.load64_lane",
                wasm_ast::MemoryInstruction::V128Store8Lane(_, _) => "v128.store8_lane",
                wasm_ast::MemoryInstruction::V128Store16Lane(_, _) => "v128.store16_lane",
                wasm_ast::MemoryInstruction::V128Store32Lane(_, _) => "v128.store32_lane",
                wasm_ast::MemoryInstruction::V128Store64Lane(_, _) => "v128.store64_lane",
                wasm_ast::MemoryInstruction::Size => "memory.size",
                wasm_ast::MemoryInstruction::Grow => "memory.grow",
                wasm_ast::MemoryInstruction::Fill => "memory.fill",
                wasm_ast::MemoryInstruction::Copy => "memory.copy",
                wasm_ast::MemoryInstruction::Init(_) => "memory.init",
                wasm_ast::MemoryInstruction::DataDrop(_) => "data.drop",
            }
        },
        Instruction::Control(i) => {
            match i {
                wasm_ast::ControlInstruction::Nop => "nop",
                wasm_ast::ControlInstruction::Unreachable => "unreachable",
                wasm_ast::ControlInstruction::Block(_, _) => "block",
                wasm_ast::ControlInstruction::Loop(_, _) => "loop",
                wasm_ast::ControlInstruction::If(_, _, _) => "if",
                wasm_ast::ControlInstruction::Branch(_) => "br",
                wasm_ast::ControlInstruction::BranchIf(_) => "br_if",
                wasm_ast::ControlInstruction::BranchTable(_, _) => "br_table",
                wasm_ast::ControlInstruction::Return => "return",
                wasm_ast::ControlInstruction::Call(_) => "call",
                wasm_ast::ControlInstruction::CallIndirect(_, _) => "call_indirect",
                // no "end" instruction here
                // no "else" instruction here
            }
        },
    };
    String::from(name_str)
}

pub fn get_dummy_module() -> Module {
    let mut dummy_module_builder = get_base_module_builder();
    dummy_module_builder.add_function_type(FunctionType::runnable()).unwrap();
    dummy_module_builder.add_function(Function::new(0, vec![ValueType::I64].into(), Expression::empty())).unwrap();
    dummy_module_builder.build()
}

pub fn get_base_module_builder() -> ModuleBuilder {
    // Module here has default sections:
    // table, memory, globals, elements, data
    let mut module_builder = Module::builder();

    let table_kind = TableType::new(ReferenceType::Function, Limit::unbounded(0x1000));
    let table = Table::new(table_kind);
    let table_idx = module_builder.add_table(table).unwrap();
    
    let memory_limit = Limit::unbounded(0x1000);
    let memory_type = MemoryType::new(memory_limit);
    // let memory = Memory::new(memory_type.clone()); // instead of adding this, import this
    // module_builder.add_memory(memory).unwrap();

    let global_type = GlobalType::mutable(ValueType ::I64);
    let global_init = vec![0i64.into()].into();
    let global = Global::new(global_type, global_init);
    let _global_idx = module_builder.add_global(global).unwrap();
    
    let element_offset: Expression = vec![0i32.into()].into();
    let element_initializers = vec![0].to_initializers();
    let element = Element::active(table_idx, element_offset.clone(), ReferenceType::Function, element_initializers.clone());
    let _element_idx = module_builder.add_element(element).unwrap();

    let data_initializer = vec![0];
    let data = Data::passive(data_initializer.clone());
    let _data_idx = module_builder.add_data(data);
    module_builder.include_data_count();

    let import_desc = ImportDescription::Memory(memory_type);
    let import = Import::new("module".into(), "mem".into(), import_desc);
    module_builder.add_import(import).unwrap();

    module_builder
}

pub fn wrap_instruction_to_module(instr: &Instruction) -> Result<Vec<Module>, Error> {
    let dummy_module = get_dummy_module();

    let instr_types_vec = match get_instruction_type(&dummy_module, 0, instr) {
        Ok(types) => types,
        Err(error) => return Err(error),
    };
    if instr_types_vec.is_empty() {
        return Err(anyhow!("Instruction type cannot be retrieved"))
    };

    let mut module_vec = Vec::new();
    for instr_type in instr_types_vec {
        let mut module_builder = get_base_module_builder();

        let params = instr_type.param_types;
        let results = instr_type.ret_types;
        let func_type = FunctionType::new(params.clone().into(), results.into());
        let _type_idx = module_builder.add_function_type(func_type).unwrap();

        let mut body_instrs: Vec<Instruction> = Vec::new();
        for localidx in 0..params.len() {
            body_instrs.push(VariableInstruction::LocalGet(localidx as u32).into());
        };
        body_instrs.push(instr.clone());
        let body = Expression::new(body_instrs);
        let locals: Vec<ValueType> = match instr {
            Instruction::Variable(VariableInstruction::LocalGet(_)) |
            Instruction::Variable(VariableInstruction::LocalSet(_)) |
            Instruction::Variable(VariableInstruction::LocalTee(_)) => {
                vec![ValueType::I64] // ensure that a local exists when instruction is local related
            },
            _ => Vec::new(),
        }; // just empty locals
        let function = Function::new(0, locals.into(), body);
        let function_idx = module_builder.add_function(function).unwrap();

        let export_name = Name::new("main".into());
        let export_desc = ExportDescription::Function(function_idx);
        module_builder.add_export(Export::new(export_name, export_desc));
        
        let module = module_builder.build();
        module_vec.push(module);
    };

    Ok(module_vec)
}

// excludes control instructions
pub fn get_instr_iterator_no_control() -> impl Iterator<Item = Instruction> {
    // TODO: we need to make sequences better
    // let numeric_iter = all::<NumericInstruction>();
    // let reference_iter = all::<ReferenceInstruction>();
    // let parametric_iter = all::<ParametricInstruction>();
    // let variable_iter = all::<VariableInstruction>();
    // let table_iter = all::<TableInstruction>();
    // let memory_iter = all::<MemoryInstruction>();
    // let control_iter = all::<ControlInstruction>();

    let instr_iter = all::<Instruction>();
    instr_iter.filter(|x| match x {
        Instruction::Control(_) => false,
        Instruction::Numeric(_) => true,
        Instruction::Reference(_) => false,
        Instruction::Parametric(_) => true,
        Instruction::Variable(_) => false,
        Instruction::Table(_) => false,
        Instruction::Memory(_) => false,
        Instruction::Vector(_) => true,
    })
}

#[cfg(test)]
mod test {
    use wasm_ast::{Module, NumericInstruction};

    use super::{get_instruction_type, get_instr_iterator_no_control};


    #[test]
    fn test_get_instruction_type_basic() {
        let instruction = NumericInstruction::Multiply(wasm_ast::NumberType::I32).into();

        let builder = Module::builder();
        let module = builder.build();
        let funcidx = 0u32;
        println!("{:?}", get_instruction_type(&module, funcidx, &instruction));
    }

    #[test]
    fn test_get_instr_iterator_no_control() {
        let instr_iter = get_instr_iterator_no_control();
        for instr in instr_iter {
            println!("{:?}", instr);
        }
    }
}