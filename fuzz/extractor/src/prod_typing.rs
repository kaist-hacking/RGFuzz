use wasm_ast::{ResultType, Module, FunctionIndex};

use crate::{prod::{ProdRule, ProdOperand, ProdToken}, wasm_comp::{get_dummy_module, get_instr_iterator_no_control, get_instruction_type}};

/// Typing rule based production rules

pub fn get_typing_rule_prods(ret_types: ResultType) -> Vec<ProdRule> {
    let dummy_module = get_dummy_module();
    get_typing_rule_prods_mod(&dummy_module, 0, ret_types)
}

pub fn get_typing_rule_prods_mod(
    module: &Module, 
    funcidx: FunctionIndex, 
    ret_types: ResultType
) -> Vec<ProdRule> {
    let mut prodrule_vec: Vec<ProdRule> = Vec::new();

    let instr_iter = get_instr_iterator_no_control();
    for instr in instr_iter {
        let mut op_arg_cnt = 0;
        let instr_type_vec = get_instruction_type(module, funcidx, &instr).unwrap();
        for instr_type in instr_type_vec {
            let instr_param_types = &instr_type.param_types;
            let instr_ret_types = &instr_type.ret_types;
            let instr_operand = if instr_type.has_operand {
                op_arg_cnt += 1;
                Some(ProdOperand::OpArg(op_arg_cnt - 1)) // Assign arg instead of wildcard
            }
            else {
                None
            };
            if instr_ret_types == ret_types.kinds() {
                let mut instrs = Vec::new();
                let mut arg_cnt = 0;
                for _ in instr_param_types {
                    instrs.push(ProdToken::Arg(arg_cnt));
                    arg_cnt += 1;
                }
                instrs.push(ProdToken::Instr(instr.clone(), instr_operand));
                
                prodrule_vec.push(ProdRule {
                    param_types: instr_param_types.clone().into(),
                    ret_types: instr_ret_types.clone().into(),
                    arg_conds: Vec::new(),
                    oparg_conds: Vec::new(),
                    instrs,
                });
                break;
            }
        }
    }
    prodrule_vec
}

#[cfg(test)]
mod test {
    use enum_iterator::all;
    use wasm_ast::ValueType;

    use super::*;

    #[test]
    fn test_get_typing_rule_prods() {
        for ty in all::<ValueType>() {
            println!("{}", "=".repeat(30));
            println!("{:#?}", get_typing_rule_prods(vec![ty].into()));
        }
    }
}