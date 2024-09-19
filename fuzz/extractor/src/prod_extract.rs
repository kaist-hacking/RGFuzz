use std::{collections::HashMap, usize};

use cranelift_codegen::ir::Opcode;
use wasm_ast::{Instruction, ValueType};

use crate::{norm::{CondExpr, MatchResult, MatchStmt, UnifiedExprIdx, UnifiedStmt}, prod::{ProdCondExpr, ProdCondition, ProdOperand, ProdRule, ProdToken}, rule_match::{match_and_learn, MatchOption}, wasm_comp::{get_dummy_module, get_instruction_type, InstructionType}};

#[derive(Clone, Debug, PartialEq)]
struct ExtractEnv {
    local_args: Vec<ValueType>,
    local_map: HashMap<usize, usize>, // map from expr idx to arg idx
    operand_arg_cnt: usize,
    operand_map: HashMap<usize, usize>, // map from expr ids to oparg idx

    arg_conds_preprocess: HashMap<usize, Vec<Vec<CondExpr>>>, // key is arg idx
    oparg_conds_preprocess: HashMap<usize, Vec<Vec<CondExpr>>>, // key is oparg idx
}

impl ExtractEnv {
    fn new() -> Self {
        Self {
            local_args: Vec::new(),
            local_map: HashMap::new(),
            operand_arg_cnt: 0,
            operand_map: HashMap::new(),

            arg_conds_preprocess: HashMap::new(),
            oparg_conds_preprocess: HashMap::new(),
        }
    }

    fn get_or_add_arg(&mut self, expr_idx: usize, ty: ValueType, conds: &Vec<Vec<CondExpr>>) -> Option<usize> {
        if self.local_map.contains_key(&expr_idx) {
            let arg_idx = *self.local_map.get(&expr_idx).unwrap();
            if self.local_args[arg_idx] != ty {
                None
            }
            else {
                Some(arg_idx)
            }
        }
        else {
            let new_arg_idx = self.local_args.len();
            self.local_args.push(ty);

            // add conds (in raw without processing)
            assert!(!self.arg_conds_preprocess.contains_key(&new_arg_idx));
            self.arg_conds_preprocess.insert(new_arg_idx, conds.clone());

            self.local_map.insert(expr_idx, new_arg_idx);
            Some(new_arg_idx)
        }
    }
    
    fn get_or_add_oparg(&mut self, expr_idx: usize, conds: &Vec<Vec<CondExpr>>) -> usize {
        if self.operand_map.contains_key(&expr_idx) {
            *self.operand_map.get(&expr_idx).unwrap()
        }
        else {
            let new_oparg_idx = self.operand_arg_cnt;
            self.operand_arg_cnt += 1;
    
            // add conds (in raw without processing)
            assert!(!self.oparg_conds_preprocess.contains_key(&new_oparg_idx));
            self.oparg_conds_preprocess.insert(new_oparg_idx, conds.clone());
            
            self.operand_map.insert(expr_idx, new_oparg_idx);
            new_oparg_idx
        }
    }

    fn convert_condexpr_to_prodcondexpr(&mut self, arg_cond: &Vec<CondExpr>, cur_idx: usize, is_operand: bool) -> Option<ProdCondExpr> {
        match &arg_cond[cur_idx] {
            CondExpr::Var => Some(ProdCondExpr::Wildcard), // Var is Wildcard
            CondExpr::Const(val) => Some(ProdCondExpr::Const(*val)),
            CondExpr::ConstPrim(sym) => Some(ProdCondExpr::ConstPrim(sym.clone())),
            CondExpr::Expr { name, params } => {
                let params_len = params.len();
                let mut new_params = Vec::new();
                for param in params {
                    match param {
                        UnifiedExprIdx::LHS(idx) => {
                            let arg_idx = if is_operand { 
                                let idx_borrow = *idx.borrow();
                                match self.operand_map.get(&idx_borrow) {
                                    Some(x) => *x,
                                    None => { // add operand if non-existent (e.g., pow2)
                                        self.operand_map.insert(idx_borrow, self.operand_arg_cnt);
                                        self.operand_arg_cnt += 1;
                                        self.operand_arg_cnt - 1
                                    }
                                }
                            } else { 
                                *self.local_map.get(&*idx.borrow())?
                            };
                            new_params.push(ProdCondExpr::Var(arg_idx));
                        },
                        UnifiedExprIdx::RHS(_) => { return None; },
                        UnifiedExprIdx::Cond(idx) => {
                            new_params.push(self.convert_condexpr_to_prodcondexpr(arg_cond, *idx.borrow(), is_operand)?);
                        },
                    }
                }
                assert!(params_len == new_params.len());
                Some(ProdCondExpr::Expr { name: name.clone(), params: new_params })
            },
            CondExpr::Ident(ident_idx) => {
                match ident_idx {
                    UnifiedExprIdx::LHS(idx) => {
                        let arg_idx = if is_operand { 
                            let idx_borrow = *idx.borrow();
                            match self.operand_map.get(&idx_borrow) {
                                Some(x) => *x,
                                None => { // add operand if non-existent (e.g., pow2)
                                    self.operand_map.insert(idx_borrow, self.operand_arg_cnt);
                                    self.operand_arg_cnt += 1;
                                    self.operand_arg_cnt - 1
                                }
                            }
                        } else { 
                            *self.local_map.get(&*idx.borrow())?
                        };
                        Some(ProdCondExpr::Var(arg_idx))
                    },
                    UnifiedExprIdx::RHS(_) => unreachable!(),
                    UnifiedExprIdx::Cond(idx) => {
                        Some(self.convert_condexpr_to_prodcondexpr(arg_cond, *idx.borrow(), is_operand)?)
                    },
                }
            },
        }
    }

    fn process_arg_conds(&mut self) -> Vec<ProdCondition> {
        let mut results = Vec::new();
        for (arg_idx, arg_conds) in self.arg_conds_preprocess.clone() {
            if arg_conds.len() == 0 {
                continue;
            }
            
            let conds = arg_conds.iter().flat_map(
                |x| self.convert_condexpr_to_prodcondexpr(x, x.len() - 1, false)
            ).collect::<Vec<_>>();

            if conds.len() == 0 {
                continue;
            }
            
            results.push(ProdCondition {
                arg: arg_idx,
                conds,
            });
        }
        results
    }

    fn process_oparg_conds(&mut self) -> Vec<ProdCondition> {
        let mut results = Vec::new();
        for (oparg_idx, oparg_conds) in self.oparg_conds_preprocess.clone() {
            if oparg_conds.len() == 0 {
                continue;
            }
            
            let conds = oparg_conds.iter().flat_map(
                |x| self.convert_condexpr_to_prodcondexpr(x, x.len() - 1, true)
            ).collect::<Vec<_>>();

            if conds.len() == 0 {
                continue;
            }

            results.push(ProdCondition {
                arg: oparg_idx,
                conds,
            });
        }
        results
    }
}

pub fn learn_prods(match_option: MatchOption) -> Vec<ProdRule> {
    let match_results = match_and_learn(match_option);
    let mut prod_rules = Vec::new();
    for match_result in match_results {
        assert!(!match_result.is_reversed());
        let mut extracted_rules = extract_prod_from_match_result(match_result);
        prod_rules.append(&mut extracted_rules);
    }
    prod_rules
}

fn extract_prod_from_match_result(match_result: MatchResult) -> Vec<ProdRule> {
    let env = ExtractEnv::new();
    let ret_type = match match_result.stmts.last().unwrap() {
        MatchStmt::Expr { data: _, instrs } => {
            let instr_type = get_instruction_type_dummy(&instrs[0]);
            let ret_types = instr_type.ret_types;
            if ret_types.is_empty() {
                None
            } else {
                assert!(ret_types.len() == 1);
                Some(ret_types[0])
            }
        },
        _ => unreachable!(),
    };
    
    let extract_results = extract_prod_rule(
        env, 
        &match_result, 
        match_result.len() - 1, 
        ret_type
    );

    let mut result_rules = Vec::new();
    for (mut env, rule) in extract_results {
        let mut new_rule = rule;
        new_rule.arg_conds = env.process_arg_conds();
        new_rule.oparg_conds = env.process_oparg_conds();
        result_rules.push(new_rule);
    }
    result_rules
}

fn extract_prod_rule(
    env: ExtractEnv, 
    match_result: &MatchResult, 
    cur_idx: usize, 
    ret_type: Option<ValueType>,
) -> Vec<(ExtractEnv, ProdRule)> {
    let mut new_env = env;
    let lhs_stmt = match_result.lhs.get(cur_idx).unwrap();
    let match_stmt = &match_result.stmts[cur_idx];
    match match_stmt {
        MatchStmt::Expr { data, instrs } => {
            let mut result_rules = Vec::new();

            // should we ignore the node? (e.g., Uextend)
            let ignore_this_instr = {
                data.opcode == Opcode::Uextend &&
                data.param_types == vec![cranelift_codegen::ir::types::I8]
            };

            for instr in instrs {
                let instr_type = get_instruction_type_dummy(instr);
                let param_types = instr_type.param_types;
                let ret_types = instr_type.ret_types;
                let params = match lhs_stmt {
                    UnifiedStmt::Expr { name: _, params } => {
                        let mut params_inner = Vec::new();
                        for param in params {
                            match param {
                                UnifiedExprIdx::LHS(inner_idx) => params_inner.push(*inner_idx.borrow()),
                                _ => { println!("{:#?}, {}", match_result, cur_idx); unreachable!() },
                            }
                        }
                        params_inner
                    },
                    _ => unreachable!(),
                };
                
                // checks: instruction validity
                if instr_type.has_operand != data.operand_type.is_some() {
                    // println!("ERROR: Instruction invalid: operand type mismatch");
                    continue;
                }
                let has_cc = data.intcc.is_some() || data.floatcc.is_some();
                if param_types.len() != data.param_types.len()  {
                    // println!("ERROR: Instruction invalid: parameter type length mismatch");
                    continue;
                }
                // first one is typevar
                if params.len() != data.param_types.len() + (instr_type.has_operand as usize) + (has_cc as usize) + 1 {
                    // println!("ERROR: Instruction invalid: parameter length mismatch");
                    continue;
                }
                if (ret_types.len() == 1) != data.ret_type.is_some() {
                    // println!("ERROR: Instruction invalid: return type mismatch");
                    continue;
                }
                if ret_type.is_some() && ret_types.len() > 0 && ret_type.unwrap() != ret_types[0] {
                    continue;
                }
                
                // skip typevar
                let mut params_iter = params.into_iter();
                let typevar_expr_idx = params_iter.next().unwrap();
                match &match_result.lhs.get(typevar_expr_idx).unwrap() {
                    UnifiedStmt::TypeVar(_) => (),
                    _ => { continue; },
                }

                // skip cc if exists
                if has_cc {
                    let cc_expr_idx = params_iter.next().unwrap();
                    match &match_result.stmts.get(cc_expr_idx).unwrap() {
                        MatchStmt::Nil => (),
                        _ => { continue; },
                    }
                }
                
                let mut params_vec = params_iter.collect::<Vec<_>>();

                // operand (at last position)
                let operand = if instr_type.has_operand {
                    let operand_expr_idx = params_vec.pop().unwrap();
                    match extract_prod_operand(&mut new_env, match_result, operand_expr_idx, ret_type) {
                        val @ Some(_) => val,
                        None => { continue; },
                    }
                } else {
                    None
                };

                // prarameters into ProdRules
                let mut param_prod_rules = vec![(
                    new_env.clone(),
                    ProdRule {
                        param_types: param_types.clone().into(), // unused
                        ret_types: ret_types.clone().into(), // unused
                        instrs: Vec::new(),
                        arg_conds: Vec::new(),
                        oparg_conds: Vec::new(),
                    }
                )];

                // reverse order for select only
                let adjusted_params_iter = 
                    if data.opcode == Opcode::Select || data.opcode == Opcode::Bitselect {
                        // first parameter (control arg) should go to last, different from wasmtime nodes
                        let control_param = params_vec.remove(0);
                        params_vec.push(control_param);
                        params_vec.into_iter()
                    }
                    else {
                        params_vec.into_iter()
                    };
                
                // construct parameters
                for (param_idx, param_ty) in adjusted_params_iter.zip(param_types) {
                    let mut temp_param_prod_rules = Vec::new();
                    for (env, rule) in param_prod_rules {
                        let result_rules = if ignore_this_instr {
                            extract_prod_rule(env, match_result, param_idx, ret_type)
                        } else {
                            extract_prod_rule(env, match_result, param_idx, Some(param_ty))
                        };

                        // filter rules based on <ret_type of param> == param_type
                        let mut filtered_rules = result_rules.into_iter().filter_map(|(filter_env, mut filter_rule)| {
                            let result_type = filter_rule.ret_types.kinds().to_vec();
                            if result_type != vec![param_ty] {
                                None
                            }
                            else {
                                let new_rule = ProdRule {
                                    param_types: filter_env.local_args.clone().into(),
                                    ret_types: ret_types.clone().into(),
                                    instrs: {
                                        let mut new_instrs = rule.instrs.clone();
                                        new_instrs.append(&mut filter_rule.instrs);
                                        new_instrs
                                    },
                                    arg_conds: Vec::new(), // later added
                                    oparg_conds: Vec::new(), // later added
                                };
                                Some((filter_env, new_rule))
                            }
                        }).collect();
                        temp_param_prod_rules.append(&mut filtered_rules);
                    }
                    param_prod_rules = temp_param_prod_rules;
                }

                // current token
                let cur_token = ProdToken::Instr(instr.clone(), operand);
                for (env, rule) in param_prod_rules {
                    result_rules.push((env.clone(), ProdRule {
                        param_types: env.local_args.into(),
                        ret_types: ret_types.clone().into(),
                        instrs: {
                            let mut new_instrs = rule.instrs;
                            // current token is added only when the instruction is not ignored
                            if !ignore_this_instr {
                                new_instrs.push(cur_token.clone());
                            }
                            new_instrs
                        },
                        arg_conds: Vec::new(), // later added
                        oparg_conds: Vec::new(), // later added
                    }));
                }
            }
            
            result_rules
        },
        MatchStmt::Arg(conds) => {
            if ret_type.is_none() {
                // println!("ERROR: ret_type should exist");
                return Vec::new(); // ret_type should exist
            }

            match new_env.get_or_add_arg(cur_idx, ret_type.unwrap(), conds) {
                Some(local_idx) => {
                    let new_rule = ProdRule {
                        param_types: new_env.local_args.clone().into(),
                        ret_types: vec![new_env.local_args[local_idx]].into(),
                        instrs: vec![ProdToken::Arg(local_idx)],
                        arg_conds: Vec::new(), // later added
                        oparg_conds: Vec::new(), // later added
                    };
                    vec![(new_env, new_rule)]
                },
                None => {
                    // println!("ERROR: get or add arg failed");
                    Vec::new() // type mismatch
                },
            }
        },
        _ => {
            println!("PROBLEM: {:#?}, {}", match_result, cur_idx);
            unreachable!()
        },
    }
}

fn extract_prod_operand(env: &mut ExtractEnv, match_result: &MatchResult, cur_idx: usize, ret_type: Option<ValueType>) -> Option<ProdOperand> {
    let match_stmt = &match_result.stmts[cur_idx];
    match match_stmt {
        MatchStmt::Arg(conds) => {
            let new_arg = env.get_or_add_oparg(cur_idx, conds);
            Some(ProdOperand::OpArg(new_arg))
        },
        MatchStmt::Const(val) => {
            if ret_type == Some(ValueType::I32) && *val >= (1 << 32) {
                None
            }
            else if ret_type == Some(ValueType::I64) && *val >= (1 << 64) {
                None
            }
            else {
                Some(ProdOperand::OpConst(*val))
            }
        },
        _ => unreachable!(),
    }
}

fn get_instruction_type_dummy(instr: &Instruction) -> InstructionType {
    let dummy_module = get_dummy_module();
    let instr_type = &get_instruction_type(&dummy_module, 0, instr).unwrap()[0];
    instr_type.clone()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_learn_prods() {
        println!("{:#?}", learn_prods(MatchOption::All));
    }

    #[test]
    fn test_learn_prods_str() {
        let prods = learn_prods(MatchOption::All);
        for prod in prods {
            println!("{:?}", prod.to_string());
        }
    }
}