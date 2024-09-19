// Step 3: Matching & Learning

use std::{collections::{HashMap, HashSet, hash_map::DefaultHasher}, fmt::Debug, hash::{Hash, Hasher}};

use anyhow::{bail, Context, Error};
use cranelift_codegen::ir::{types, Type};

use crate::{isle::ISLEParseOptions, isle_cond::process_conds, isle_subst::{check_subst_applicable, subst_apply_result, subst_result}, isle_type::type_rules_opt, norm::{LinExprIdx, LinVec, MatchResult, MatchStmt, UnifiedExprIdx, UnifiedResult, UnifiedRule, UnifiedStmt}, wasm_map::get_clir_wasm_map, wasm_norm::wasm_pair_to_matched_result};

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum MatchOption {
    All,
    Opt,
    Lower,
    X64,
    ARM64,
    S390X,
    RISCV64,
    TestOpt,
    TestLower,
}

#[derive(Debug)]
struct MatchContext {
    // name -> [ MatchResult ]
    // expressions in this map is ensured to be reachable from WASM
    match_map: HashMap<String, Vec<MatchResult>>,

    // name -> [ MatchResult ]
    // separate store that is learned from matching
    learn_map: HashMap<String, Vec<MatchResult>>,
    learn_set: HashMap<String, HashSet<u64>>, // checking duplicates

    // name -> [ UnifiedRule ]
    // rules in this map encodes substitution rules between UnifiedExprs
    subst_map: HashMap<String, Vec<UnifiedResult>>,
}

impl MatchContext {
    fn new() -> Self {
        Self {
            match_map: HashMap::new(),
            learn_map: HashMap::new(),
            learn_set: HashMap::new(),
            subst_map: HashMap::new(),
        }
    }

    fn add_expr(&mut self, name: String, match_result: MatchResult) {
        assert!(match_result.is_reversed());
        self.match_map.entry(name).or_default().push(match_result);
    }

    // fn add_learn(&mut self, name: String, learn_result: MatchResult) {
    //     assert!(learn_result.is_reversed());
    //     self.learn_map.entry(name).or_default().push(learn_result);
    // }

    fn add_subst(&mut self, name: String, lin_result: UnifiedResult) {
        self.subst_map.entry(name).or_default().push(lin_result);
    }
}

fn get_hash(str: String) -> u64 {
    let mut hasher = DefaultHasher::new();
    str.hash(&mut hasher);
    hasher.finish()
}

fn match_and_learn_prepare(match_option: MatchOption) -> (MatchContext, Vec<UnifiedRule>, Vec<UnifiedRule>) {
    let mut clir_results = Vec::new();
    match match_option {
        MatchOption::All => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::Opt));
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::Lower));
        },
        MatchOption::Opt => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::Opt));
        },
        MatchOption::Lower => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::Lower));
        },
        MatchOption::X64 => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::X64));
        },
        MatchOption::ARM64 => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::ARM64));
        },
        MatchOption::S390X => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::S390X));
        },
        MatchOption::RISCV64 => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::RISCV64));
        },
        MatchOption::TestOpt => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::TestOpt));
        },
        MatchOption::TestLower => {
            clir_results.append(&mut type_rules_opt(ISLEParseOptions::TestLower));
        },
    }
    let clir_wasm_map = get_clir_wasm_map();

    // construct context from wasm
    let mut context = MatchContext::new();
    for (irdata, instrs) in clir_wasm_map {
        match wasm_pair_to_matched_result(&irdata, &instrs) {
            Some(x) => context.add_expr(irdata.opcode.to_string(), x),
            None => (),
        }
    }

    // from Cranelift rules
    let mut clir_simplify_results = Vec::new();
    let mut clir_lower_results = Vec::new();
    for lin_result in clir_results {
        let lhs_stmt = lin_result.rule.lhs.get(lin_result.rule.lhs.len() - 1).unwrap();
        match lhs_stmt {
            UnifiedStmt::Expr { name, params} => {
                if name == "simplify" {
                    assert!(params.len() == 1);
                    let mut new_rule = lin_result.clone();
                    new_rule.remove(LinExprIdx::LHS(lin_result.rule.lhs.get_idx_ref(lin_result.rule.lhs.len() - 1).unwrap().clone()));
                    clir_simplify_results.push(new_rule);
                }
                else if name == "lower" {
                    assert!(params.len() == 1);
                    let mut new_rule = lin_result.clone();
                    new_rule.remove(LinExprIdx::LHS(lin_result.rule.lhs.get_idx_ref(lin_result.rule.lhs.len() - 1).unwrap().clone()));
                    clir_lower_results.push(new_rule);
                }
                else {
                    context.add_subst(name.clone(), lin_result);
                }
            },
            _ => (), // ignore
        }
    }

    // Process 1: substitute simplify and lowering rules with normal rules
    let mut subst_simplify_results = Vec::new();
    for clir_simplify_result in clir_simplify_results {
        let mut subst_result = subst_result(&context.subst_map, clir_simplify_result, false).unwrap_or(Vec::new());
        subst_simplify_results.append(&mut subst_result);
    }
    let mut subst_lower_results = Vec::new();
    for clir_lower_result in clir_lower_results {
        let mut subst_result = subst_result(&context.subst_map, clir_lower_result, true).unwrap_or(Vec::new());
        subst_lower_results.append(&mut subst_result);
    }

    // Process 2: process conditions and embed them to variables
    let clir_simplify_rules = subst_simplify_results.into_iter().flat_map(process_conds).collect();
    let clir_lower_rules = subst_lower_results.into_iter().flat_map(process_conds).collect();

    (context, clir_simplify_rules, clir_lower_rules)
}

pub fn match_and_learn(match_option: MatchOption) -> Vec<MatchResult> {
    let (mut context, clir_simplify_rules, clir_lower_rules) = match_and_learn_prepare(match_option);

    let mut results = Vec::new();
    let mut results_set = HashSet::new();

    // Step 1: match and learn opt. rules
    let mut is_done = false;
    let mut learned_cnt = 0;
    let mut remaining_rules = clir_simplify_rules;
    while !is_done {
        let mut cur_learned_cnt = 0;
        let mut temp_rules = Vec::new();

        for rule in &remaining_rules {
            match match_rule(&context, rule) {
                Ok((match_results, learn_used)) => {
                    if match_results.len() == 0 {
                        temp_rules.push(rule.clone());
                    }
                    else if learn_used {
                        temp_rules.push(rule.clone());
                    }

                    // do not learn rules that are from learned rules
                    // save rhs for latter matching (learning)
                    let reversed_results = match_results.iter()
                        .filter(|x| !x.from_learned)
                        .map(|x| x.deep_clone().reverse())
                        .filter(check_match_result_learnable).collect::<Vec<_>>();
                    
                    for reversed_result in reversed_results {
                        assert!(reversed_result.is_reversed());
                        let is_added = context.learn_set.entry(reversed_result.get_name()).or_default().insert(get_hash(format!("{:?}", reversed_result)));
                        if is_added {
                            context.learn_map.entry(reversed_result.get_name()).or_default().push(reversed_result);
                        }
                        cur_learned_cnt += 1;
                    }

                    for match_result in match_results {
                        let is_added = results_set.insert(format!("{:?}", match_result));
                        if is_added {
                            results.push(match_result);
                        }
                    }
                },
                Err(_) => {
                    temp_rules.push(rule.clone());
                },
            }
        }
        if learned_cnt == cur_learned_cnt && remaining_rules.len() == temp_rules.len() {
            is_done = true;
        }
        else {
            learned_cnt = cur_learned_cnt;
            remaining_rules = temp_rules;
        }
    }

    // Step 2: match and learn lowering rules
    for rule in &clir_lower_rules {
        match match_rule(&context, rule) {
            Ok((match_results, _)) => {
                for match_result in match_results {
                    let is_added = results_set.insert(format!("{:?}", &match_result));
                    if is_added {
                        results.push(match_result);
                    }
                }
            },
            Err(_) => {},
        }
    }
    
    results
}

fn match_rule(context: &MatchContext, rule: &UnifiedRule) -> Result<(Vec<MatchResult>, bool), Error> {
    // find candidates for each expr
    let mut learn_used = false;
    let mut candi_match_map: HashMap<String, Vec<&MatchResult>> = HashMap::new();
    let mut candi_learn_map: HashMap<String, Vec<MatchResult>> = HashMap::new();
    
    fn fetch_learn_map(context: &MatchContext, rule: &UnifiedRule, fetch_name: &String, candi_learn_map: &mut HashMap<String, Vec<MatchResult>>) {
        assert!(!candi_learn_map.contains_key(fetch_name));
        candi_learn_map.insert(fetch_name.clone(), Vec::new());
        for cur_idx in 0..rule.lhs.len() {
            let cur_stmt = rule.lhs.get(cur_idx).unwrap();
            let cur_result = MatchResult::new(rule.deep_clone());
            match cur_stmt {
                UnifiedStmt::Expr { name, .. } if name == fetch_name => {
                    let learn_candidates = context.learn_map.get(name);
                    if learn_candidates.is_some() {
                        for candidate in learn_candidates.unwrap() {
                            if check_match_applicable(&cur_result, candidate, cur_idx) {
                                candi_learn_map.get_mut(fetch_name).unwrap().push(candidate.deep_clone());
                            }
                        }
                    }
                },
                UnifiedStmt::Ident(_) => unreachable!(),
                _ => (),
            }
        }
    }

    for cur_idx in 0..rule.lhs.len() {
        let cur_stmt = rule.lhs.get(cur_idx).unwrap();
        let cur_result = MatchResult::new(rule.deep_clone());
        match cur_stmt {
            UnifiedStmt::Expr { name, .. } => {
                let match_candidates = context.match_map.get(name);
                if match_candidates.is_some() {
                    for candidate in match_candidates.unwrap() {
                        if check_match_applicable(&cur_result, candidate, cur_idx) {
                            candi_match_map.entry(name.clone()).or_default().push(candidate);
                        }
                    }
                }
                // lazy fetch learn candidates
            },
            UnifiedStmt::Ident(_) => unreachable!(),
            _ => (),
        }
    }

    // repeatedly match and substitute rule
    let mut pending_vec = vec![MatchResult::new(rule.deep_clone())];
    let mut pending_set = HashSet::new();
    pending_set.insert(get_hash(format!("{:?}", pending_vec[0])));
    let mut complete_vec = Vec::new();
    let mut complete_set = HashSet::new();
    while pending_vec.len() > 0 {
        let mut cur_result = pending_vec.pop().unwrap();
        assert!(!cur_result.is_reversed());

        // search from last (icmp with IntCC.xx can be handled with this method)
        let cur_idx = match cur_result.stmts.iter().rev().position(|x| x == &MatchStmt::None) {
            Some(x) => cur_result.stmts.len() - 1 - x,
            None => {
                let is_added = complete_set.insert(get_hash(format!("{:?}", &cur_result)));
                if is_added {
                    complete_vec.push(cur_result);
                }
                continue;
            },
        };
        
        // apply candidates
        let cur_stmt = cur_result.lhs.get(cur_idx).unwrap();
        match cur_stmt {
            UnifiedStmt::Var(conds) => { 
                cur_result.stmts[cur_idx] = MatchStmt::Arg(conds.clone());
                let is_added = pending_set.insert(get_hash(format!("{:?}", &cur_result)));
                if is_added {
                    pending_vec.push(cur_result);
                }
            },
            UnifiedStmt::TypeVar(_) => { 
                cur_result.stmts[cur_idx] = MatchStmt::Nil;
                let is_added = pending_set.insert(get_hash(format!("{:?}", &cur_result)));
                if is_added {
                    pending_vec.push(cur_result);
                }
            },
            UnifiedStmt::Const(val) => {
                cur_result.stmts[cur_idx] = MatchStmt::Const(*val);
                let is_added = pending_set.insert(get_hash(format!("{:?}", &cur_result)));
                if is_added {
                    pending_vec.push(cur_result);
                }
            },
            UnifiedStmt::ConstPrim(_) => { bail!("ConstPrim cannot be matched"); },
            UnifiedStmt::Expr { name, .. } => {
                let match_candidates = candi_match_map.get(name);
                let mut is_matched = false;
                if match_candidates.is_some() {
                    for candidate in match_candidates.unwrap() {
                        match match_and_subst(&cur_result, candidate, cur_idx) {
                            Ok(Some(x)) => {
                                if check_match_result_types(&x) {
                                    let is_added = pending_set.insert(get_hash(format!("{:?}", &x)));
                                    if is_added {
                                        pending_vec.push(x);
                                    }
                                    is_matched = true;
                                }
                            },
                            _ => (),
                        }
                    }
                }

                // ignore (it should be matched with wasm rules - problematic in learning)
                let learn_ban_list = vec!["icmp"];
                if !is_matched && !learn_ban_list.contains(&name.as_str()) {
                    if !candi_learn_map.contains_key(name) {
                        fetch_learn_map(context, rule, name, &mut candi_learn_map);
                    }

                    let learn_candidates = candi_learn_map.get(name);
                    if learn_candidates.is_some() {
                        for candidate in learn_candidates.unwrap() {
                            match match_and_subst(&cur_result, candidate, cur_idx) {
                                Ok(Some(mut x)) => {
                                    if check_match_result_types(&x) {
                                        let is_added = pending_set.insert(get_hash(format!("{:?}", &x)));
                                        if is_added {
                                            x.from_learned = true;
                                            pending_vec.push(x);
                                        }
                                        learn_used = true;
                                    }
                                },
                                _ => (),
                            }
                        }
                    }
                }
            },
            UnifiedStmt::Ident(_) => unreachable!(),
        }
    }

    // filter with type checking
    complete_vec = complete_vec.into_iter().collect();
    Ok((complete_vec, learn_used))
}

fn match_and_subst(match_what: &MatchResult, match_with: &MatchResult, match_idx: usize) -> Result<Option<MatchResult>, Error> {
    assert!(!match_what.is_reversed() && match_with.is_reversed());
    let (rule_what, stmts_what) = match_what.deep_clone().get_pair();
    let (rule_with, stmts_with) = match_with.deep_clone().get_pair();

    // UnifiedResults to use subst_apply_result
    let stmt_idx = UnifiedExprIdx::LHS(rule_what.lhs.get_idx_ref(match_idx).context("Invalid match_idx")?.clone());
    let lin_result_what = UnifiedResult { rule: rule_what, cond_stmts: LinVec::new(), cond_pairs: Vec::new() };
    let lin_result_with = UnifiedResult { rule: rule_with, cond_stmts: LinVec::new(), cond_pairs: Vec::new() };

    let mut stmts_result = stmts_what.clone();
    match subst_apply_result(&lin_result_what, &lin_result_with, &stmt_idx, &mut stmts_result, &stmts_with)? {
        Some(rule_result) => {
            assert!(rule_result.rule.lhs.len() == stmts_result.len());
            let mut result = MatchResult::new_with_stmts(rule_result.rule, stmts_result);
            result.from_learned = match_what.from_learned;
            Ok(Some(result))
        },
        _ => Ok(None),
    }
}

fn check_match_applicable(match_what: &MatchResult, match_with: &MatchResult, match_idx: usize) -> bool {
    assert!(!match_what.is_reversed() && match_with.is_reversed());
    let (rule_what, _) = match_what.deep_clone().get_pair();
    let (rule_with, _) = match_with.deep_clone().get_pair();

    // UnifiedResults to use subst_apply_result
    let stmt_idx = UnifiedExprIdx::LHS(rule_what.lhs.get_idx_ref(match_idx).unwrap().clone());
    let lin_result_what = UnifiedResult { rule: rule_what, cond_stmts: LinVec::new(), cond_pairs: Vec::new() };
    let lin_result_with = UnifiedResult { rule: rule_with, cond_stmts: LinVec::new(), cond_pairs: Vec::new() };

    check_subst_applicable(&lin_result_what, &lin_result_with, &stmt_idx)
}

// simply check match data types
fn check_match_result_types(match_result: &MatchResult) -> bool {
    fn check_match_result_types_rec(match_result: &MatchResult, cur_ty: Option<Type>, cur_idx: usize) -> bool {
        let match_stmt = &match_result.stmts[cur_idx];
        let lhs_stmt = match_result.lhs.get(cur_idx).unwrap();
        let type_check = match match_stmt {
            MatchStmt::Expr { data, .. } => {
                match lhs_stmt {
                    UnifiedStmt::Expr { name, .. } => {
                        data.ret_type == cur_ty ||
                        (name == "icmp" && data.ret_type == Some(types::I8) && cur_ty.is_some() && 
                            vec![types::I16, types::I32, types::I64].contains(&cur_ty.unwrap()))
                    },
                    _ => data.ret_type == cur_ty,
                }
            },
            _ => true,
        };

        match (match_stmt, lhs_stmt) {
            (MatchStmt::Expr { data, .. }, UnifiedStmt::Expr { name: _, params }) => {
                let mut params_iter = params.iter().skip(1); // skip typevar
                if data.operand_type.is_some() {
                    let _ = params_iter.next(); // discard operand
                }
                if data.intcc.is_some() || data.floatcc.is_some() {
                    let _ = params_iter.next(); // discard cc
                }

                assert!(params_iter.len() == data.param_types.len());
                let check_result = params_iter.zip(data.param_types.iter()).all(|(idx, ty)| {
                    let new_idx = match idx {
                        UnifiedExprIdx::LHS(inner_idx) => *inner_idx.borrow(),
                        _ => unreachable!(),
                    };
                    check_match_result_types_rec(match_result, Some(ty.clone()), new_idx)
                });
                type_check && check_result
            },
            _ => type_check,
        }
    }

    let init_idx = match_result.lhs.len() - 1;
    let init_ty = match match_result.stmts.last().unwrap() {
        MatchStmt::Expr { data, .. } => data.ret_type,
        _ => unreachable!(),
    };
    check_match_result_types_rec(match_result, init_ty, init_idx)
}

// something like x @ .. -> .. x .. is not allowed in the reversed result!
// such things would cause infinite loop, so these are removed from the learning process
fn check_match_result_learnable(match_result: &MatchResult) -> bool {
    assert!(match_result.is_reversed());

    fn is_last_of_lhs(idx: &LinExprIdx, lhs_len: usize) -> bool {
        match idx {
            LinExprIdx::LHS(inner_idx) => {
                if *inner_idx.borrow() == (lhs_len - 1) {
                    true
                }
                else {

                    false
                }
            },
            _ => false,
        }
    }

    // iterate rhs
    let last_lhs_expr_name = match match_result.lhs.get(match_result.lhs.len() - 1).unwrap() {
        UnifiedStmt::Expr { name, .. } => Some(name.clone()),
        _ => None
    };
    for idx in 0..match_result.rhs.len() {
        let rhs_stmt = match_result.rhs.get(idx).unwrap();
        match rhs_stmt {
            UnifiedStmt::Expr { name, params } => {
                for param in params {
                    if is_last_of_lhs(param, match_result.lhs.len()) {
                        return false;
                    }
                }
            },
            UnifiedStmt::Ident(ident_idx) => {
                if is_last_of_lhs(ident_idx, match_result.lhs.len()) {
                    return false;
                }
            },
            _ => (),
        }
    }
    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_match_and_learn_prepare() {
        println!("{:#?}", match_and_learn_prepare(MatchOption::All).1);
    }

    #[test]
    fn test_match_and_learn() {
        println!("{:#?}", match_and_learn(MatchOption::All));
    }
}