use std::collections::HashMap;
use std::env;
use std::process;

use anyhow::{Error, anyhow};
use cranelift_codegen::ir::Block;
use cranelift_codegen::ir::Function;
use cranelift_codegen::ir::Inst;
use cranelift_codegen::isa::{CallConv, TargetFrontendConfig};
use cranelift_wasm::translate_module;
use cranelift_wasm::DummyEnvironment;
use target_lexicon::PointerWidth;

/// Translates single instruction WASM module into CLIR (Cranelift IR)
fn translate_wasm_module(data: &[u8]) -> Result<Vec<Function>, Error> {
    let mut dummy_environment = DummyEnvironment::new(TargetFrontendConfig {
        default_call_conv: CallConv::SystemV,
        pointer_width: PointerWidth::U64,
    });

    match translate_module(data, &mut dummy_environment) {
        Ok(_) => Ok(dummy_environment.info.function_bodies.into_iter().map(|(_, f)| f).collect()),
        Err(error) => Err(anyhow!("translate_data failed with {:?}", error)),
    }
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

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("[*] {} <filename>", args[0]);
        process::exit(1);
    }

    // file read
    let filename = &args[1];
    let data = std::fs::read(filename).unwrap();
    let ir_funcs = translate_wasm_module(data.as_slice()).unwrap();

    // count IR expressions
    let mut count_map = HashMap::new();
    for func in ir_funcs.iter() {
        for block in &func.layout {
            let insts = postprocess_block(func, &block);
            for inst in insts {
                let inst_data = func.dfg.insts[inst];
                let opcode = inst_data.opcode().to_string();
                let intcc = inst_data.cond_code();
                let floatcc = inst_data.fp_cond_code();
                let inst_args = inst_data.arguments(&func.dfg.value_lists);
                let param_types = inst_args.into_iter().map(|arg| func.dfg.value_type(arg.clone())).collect::<Vec<_>>();
                let ret_type = func.dfg.inst_result_types(inst, func.dfg.ctrl_typevar(inst)).next();
                
                let mut ir_str = String::new();
                ir_str += &opcode;
                if intcc.is_some() {
                    ir_str += ".";
                    ir_str += &format!("{:?}", intcc.unwrap());
                }
                if floatcc.is_some() {
                    ir_str += ".";
                    ir_str += &format!("{:?}", floatcc.unwrap());
                }
                if opcode != "call" && opcode != "call_indirect" {
                    ir_str += "-[";
                    ir_str += &param_types.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(",");
                    ir_str += "]-[";
                    ir_str += &ret_type.map(|t| t.to_string()).unwrap_or("".to_string());
                    ir_str += "]";
                }

                let count = count_map.entry(ir_str).or_insert(0);
                *count += 1;
            }
        }
    }

    println!("{:?}", count_map);
}