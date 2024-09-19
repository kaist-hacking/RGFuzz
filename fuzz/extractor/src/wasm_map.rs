// Step 2.1: Map each WASM instruction to Cranelift IR

use std::collections::HashMap;

use anyhow::{Error, anyhow};
use cranelift_wasm::{DummyEnvironment, DefinedFuncIndex, translate_module};
use cranelift_codegen::{ir::{function::Function, Block, InstructionData}, isa::{TargetFrontendConfig, CallConv}, ir::{entities::Inst, instructions::Opcode, types::Type, condcodes::{IntCC, FloatCC}}};
use target_lexicon::PointerWidth;
use wasm_ast::{Instruction, emitter};

use crate::wasm_comp::{wrap_instruction_to_module, get_instr_iterator_no_control};

/// Translates single instruction WASM module into CLIR (Cranelift IR)
fn translate_wasm_module(data: &[u8]) -> Result<Function, Error> {
    let mut dummy_environment = DummyEnvironment::new(TargetFrontendConfig {
        default_call_conv: CallConv::SystemV,
        pointer_width: PointerWidth::U64,
    });

    match translate_module(data, &mut dummy_environment) {
        Ok(_) => Ok(dummy_environment.info.function_bodies.get(DefinedFuncIndex::from_u32(0)).unwrap().clone()),
        Err(error) => Err(anyhow!("translate_data failed with {:?}", error)),
    }
}

fn translate_single_instr_module(instr: &Instruction) -> Result<Vec<Function>, Error> {
    let module_vec = match wrap_instruction_to_module(instr) {
        Ok(modules) => modules,
        Err(error) => return Err(error),
    };

    let mut function_vec: Vec<Function> = Vec::new();
    for module in module_vec {
        let mut data: Vec<u8> = Vec::new();
        let _bin_size = emitter::emit_binary(&module, &mut data).unwrap();
        
        match translate_wasm_module(data.as_slice()) {
            Ok(function) => function_vec.push(function),
            Err(error) => return Err(error),
        }
    }
    Ok(function_vec)
}

#[derive(Clone, Debug, PartialEq, Copy, Eq, Hash)]
pub enum IROperandType {
    U8,
    Imm64,
    Uimm32,
    // Uimm64,
    V128Imm,
    Ieee32,
    Ieee64,
    Offset32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IRData {
    pub opcode: Opcode,
    pub intcc: Option<IntCC>,
    pub floatcc: Option<FloatCC>,
    pub operand_type: Option<IROperandType>,
    pub param_types: Vec<Type>,
    pub ret_type: Option<Type>,
}

fn postprocess_function(func: &Function) -> Vec<IRData> {
    let mut insts: Vec<Inst> = Vec::new();
    for block in &func.layout {
        let mut block_insts = postprocess_block(func, &block);
        insts.append(&mut block_insts);
    };
    
    let mut irdata_vec:Vec<IRData> = Vec::new();
    for inst in insts {
        let inst_data = func.dfg.insts[inst.clone()];
        let opcode = inst_data.opcode();
        let intcc = inst_data.cond_code();
        let floatcc = inst_data.fp_cond_code();
        let value_pool_ref = &func.dfg.value_lists;
        let inst_args = inst_data.arguments(value_pool_ref);
        let param_types = inst_args.into_iter().map(|arg| func.dfg.value_type(arg.clone())).collect();
        let ret_type = func.dfg.inst_result_types(inst, func.dfg.ctrl_typevar(inst)).next();
        let operand_type = match inst_data {
            InstructionData::BinaryImm8 { .. } |
            InstructionData::TernaryImm8 { .. } => Some(IROperandType::U8),

            InstructionData::BinaryImm64 { .. } |
            InstructionData::IntCompareImm { .. } |
            InstructionData::UnaryImm { .. } => Some(IROperandType::Imm64),

            InstructionData::Shuffle { .. } => Some(IROperandType::V128Imm),

            InstructionData::UnaryIeee32 { .. } => Some(IROperandType::Ieee32),
            InstructionData::UnaryIeee64 { .. } => Some(IROperandType::Ieee64),

            InstructionData::Load { .. } |
            InstructionData::StackLoad { .. } |
            InstructionData::StackStore { .. } |
            InstructionData::Store { .. } |
            InstructionData::StoreNoOffset { .. } |
            // InstructionData::TableAddr { .. } => Some(IROperandType::Offset32), // remove this case for latest version (20.0.0)
            
            InstructionData::UnaryConst {.. } => Some(IROperandType::Uimm32),

            _ => None
        };
        irdata_vec.push(IRData {
            opcode,
            intcc,
            floatcc,
            operand_type,
            param_types,
            ret_type,
        });
    }
    irdata_vec
}

fn postprocess_block(func: &Function, block: &Block) -> Vec<Inst> {
    let mut insts: Vec<Inst> = Vec::new();
    for inst in func.layout.block_insts(block.clone()) {
        let inst_data = func.dfg.insts[inst];
        if !inst_data.opcode().is_branch() && !inst_data.opcode().is_return() {
            insts.push(inst);
        }
    };
    insts
}

pub fn get_clir_wasm_map() -> HashMap<IRData, Vec<Instruction>> {
    let mut clir_wasm_idx_map: HashMap<IRData, Vec<usize>> = HashMap::new();
    let mut wasm_idx_clir_map: HashMap<usize, Vec<IRData>> = HashMap::new();
    let mut idx_wasm_map: HashMap<usize, Instruction> = HashMap::new();

    // construct maps
    let instr_iter = get_instr_iterator_no_control();
    for (idx, instr) in instr_iter.enumerate() {
        idx_wasm_map.insert(idx, instr.clone());

        let function_vec = translate_single_instr_module(&instr).unwrap();
        for function in function_vec {
            let irdata_vec = postprocess_function(&function);
            wasm_idx_clir_map.insert(idx, irdata_vec.clone());
            for irdata in irdata_vec {
                let idx_vec = clir_wasm_idx_map.get_mut(&irdata);
                match idx_vec {
                    Some(v) => v.push(idx),
                    None => {
                        clir_wasm_idx_map.insert(irdata, vec![idx]);
                        ()
                    },
                };
            }
        }
    }
    
    // If an IRData corresponds to multiple instructions, prioritize instructions that translate
    // to smaller number of IRs. (Heuristic)
    // Remove instructions with low priority from the CLIR -> WASM map
    let mut clir_wasm_map: HashMap<IRData, Vec<Instruction>> = HashMap::new();
    for (irdata, idx_vec) in clir_wasm_idx_map {
        assert!(idx_vec.len() > 0);
        let instr_vec = idx_vec.clone().into_iter().map(|idx| idx_wasm_map.get(&idx).unwrap().clone()).collect();
        if idx_vec.clone().len() == 1 {
            clir_wasm_map.insert(irdata, instr_vec);
            continue;
        }

        let ir_len_vec = idx_vec.clone().into_iter().map(|idx| wasm_idx_clir_map.get(&idx).unwrap().len());
        let min_size = ir_len_vec.clone().min().unwrap();
        let mut filtered_instr_vec = Vec::new();
        for (idx, len) in ir_len_vec.enumerate() {
            if len == min_size {
                filtered_instr_vec.push(instr_vec.get(idx).unwrap().clone());
            }
        }
        clir_wasm_map.insert(irdata, filtered_instr_vec);
    }

    // Testing code for non-removal of low-priority instructions
    /*
    let mut clir_wasm_map: HashMap<IRData, Vec<Instruction>> = HashMap::new();
    for (irdata, idx_vec) in clir_wasm_idx_map {
        assert!(idx_vec.len() > 0);
        let instr_vec = idx_vec.clone().into_iter().map(|idx| idx_wasm_map.get(&idx).unwrap().clone()).collect();
        clir_wasm_map.insert(irdata, instr_vec);
    }
    */

    clir_wasm_map
}

#[cfg(test)]
mod test {
    use wasm_ast::{NumericInstruction, Module, ValueType, Function, Expression, sections::emit_module, FunctionType, ResultType};

    use crate::wasm_comp::get_instr_iterator_no_control;

    use super::*;

    #[test]
    fn test_translate_single_manual() {
        // function
        let body: Expression = vec![
            32i32.into(),
            2i32.into(),
            NumericInstruction::Multiply(wasm_ast::NumberType::I32).into()
        ].into();
        let function = Function::new(0, vec![].into(), body.clone());
        let params = ResultType::empty();
        let rets = ResultType::from(vec![ValueType::I32]);
        let function_type = FunctionType::new(params, rets);

        let mut builder = Module::builder();
        let _ = builder.add_function_type(function_type);
        let _ = builder.add_function(function);
        let module = builder.build();
        let mut output = Vec::new();
        let _ = emit_module(&module, &mut output);
        println!("{:?}", output);
        println!("{:#?}", translate_wasm_module(&output));
    }

    #[test]
    fn test_translate_single_instr_module() {
        let instr: Instruction = NumericInstruction::Multiply(wasm_ast::NumberType::I32).into();
        println!("{:?}", translate_single_instr_module(&instr));
    }

    #[test]
    fn test_translate_single_instr_module_enumerative() {
        let instr_iter = get_instr_iterator_no_control();
        for instr in instr_iter {
            println!("{}", "=".repeat(30));
            println!("{:?}", instr.clone());
            let function_vec = translate_single_instr_module(&instr);
            println!("{:?}", &function_vec);
            for function in function_vec.unwrap() {
                let insts = postprocess_function(&function);
                println!("{:#?}", insts);
            }
        }
    }

    #[test]
    fn test_get_clir_wasm_map() {
        let map = get_clir_wasm_map();
        println!("{:#?}", map);
    }
}