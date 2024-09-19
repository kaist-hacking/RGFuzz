// Step 1.4.2: Process rule conditions of linearized ISLE rules

use std::{cell::RefCell, rc::Rc};

use crate::norm::{CondExpr, LinExpr, LinExprIdx, LinResult, LinRule};

// process conditions to have (Var -> _) form
pub fn process_conds(lin_result: LinResult) -> Vec<LinRule> {
    let cond_pairs = lin_result.cond_pairs.clone();
    let mut results = vec![lin_result];
    for (cond_pair_fst, cond_pair_snd) in cond_pairs {
        results = results.into_iter().flat_map(
            |result| process_rule_cond_pair(result, &cond_pair_fst, &cond_pair_snd)
        ).collect();
    }
    results.into_iter().map(|x| x.rule).collect()
}

fn process_rule_cond_pair(lin_result: LinResult, cond_pair_fst: &LinExprIdx, cond_pair_snd: &LinExprIdx) -> Vec<LinResult> {
    let mut new_lin_result = lin_result;
    let stmt_fst = new_lin_result.get_mut(cond_pair_fst).unwrap().clone();
    let stmt_snd = new_lin_result.get_mut(cond_pair_snd).unwrap().clone();
    match (stmt_fst, stmt_snd) {
        (LinExpr::TypeVar(_), _) | (_, LinExpr::TypeVar(_)) => (), // ignore
        (LinExpr::Ident(ident_idx), _) => {
            return process_rule_cond_pair(new_lin_result, &ident_idx, cond_pair_snd);
        },
        (_, LinExpr::Ident(ident_idx)) => {
            return process_rule_cond_pair(new_lin_result, cond_pair_fst, &ident_idx);
        },
        (LinExpr::Var(_), LinExpr::Var(_)) => { // Pattern: (Var, Var) -> merge two vars
            merge_two_vars(&mut new_lin_result, cond_pair_fst, cond_pair_snd);
        },
        (LinExpr::Var(_), x @ LinExpr::Const(_)) |
        (LinExpr::Var(_), x @ LinExpr::ConstPrim(_)) => {
            *new_lin_result.get_mut(cond_pair_fst).unwrap() = x;
        },
        (x @ LinExpr::Const(_), LinExpr::Var(_)) |
        (x @ LinExpr::ConstPrim(_), LinExpr::Var(_)) => {
            *new_lin_result.get_mut(cond_pair_snd).unwrap() = x;
        },
        (LinExpr::ConstPrim(sym), LinExpr::Expr { name, params }) |
        (LinExpr::Expr { name, params }, LinExpr::ConstPrim(sym)) 
            if sym == "true" || sym == "false" => { // for other syms, ignore
            return handle_bool_expr(new_lin_result, sym == "true", name, params);
        },
        (LinExpr::Var(_), LinExpr::Expr { name, .. }) => {
            if name == "iconst" {
                copy_expr_to_var(&mut new_lin_result, cond_pair_fst, cond_pair_snd);
            }
            else {
                let mut new_conds_vec = Vec::new();
                convert_to_condexpr(&mut new_conds_vec, &new_lin_result, cond_pair_snd);
    
                // need to retrieve the expression again since stmts are cloned
                let stmt_fst = new_lin_result.get_mut(cond_pair_fst).unwrap();
                match stmt_fst {
                    LinExpr::Var(ref mut conds) => {
                        conds.push(new_conds_vec);
                    },
                    _ => unreachable!(),
                }
            }
        },
        (LinExpr::Expr { name, .. }, LinExpr::Var(_)) => {
            if name == "iconst" {
                copy_expr_to_var(&mut new_lin_result, cond_pair_snd, cond_pair_fst);
            }
            else {
                let mut new_conds_vec = Vec::new();
                convert_to_condexpr(&mut new_conds_vec, &new_lin_result, cond_pair_fst);

                // need to retrieve the expression again since stmts are cloned
                let stmt_fst = new_lin_result.get_mut(cond_pair_snd).unwrap();
                match stmt_fst {
                    LinExpr::Var(ref mut conds) => {
                        conds.push(new_conds_vec);
                    },
                    _ => unreachable!(),
                }
            }
        },
        _ => (), // ignore
    }
    
    vec![new_lin_result]
}

fn merge_two_vars(lin_result: &mut LinResult, var1: &LinExprIdx, var2: &LinExprIdx) {
    match (var1, var2) {
        (LinExprIdx::LHS(idx1), LinExprIdx::LHS(idx2)) => {
            // merge to smaller one
            let idx_val1 = *idx1.borrow();
            let idx_val2 = *idx2.borrow();
            if idx_val1 < idx_val2 {
                lin_result.remove_and_subst(var2.clone(), var1.clone());
            }
            else {
                lin_result.remove_and_subst(var1.clone(), var2.clone());
            }
        },
        (LinExprIdx::LHS(_), _) => {
            lin_result.remove_and_subst(var2.clone(), var1.clone());
        },
        (_, LinExprIdx::LHS(_)) => {
            lin_result.remove_and_subst(var1.clone(), var2.clone());
        },
        (LinExprIdx::RHS(idx1), LinExprIdx::RHS(idx2)) => {
            // merge to smaller one
            let idx_val1 = *idx1.borrow();
            let idx_val2 = *idx2.borrow();
            if idx_val1 < idx_val2 {
                lin_result.remove_and_subst(var2.clone(), var1.clone());
            }
            else {
                lin_result.remove_and_subst(var1.clone(), var2.clone());
            }
        },
        (LinExprIdx::RHS(_), _) => { // DO NOT MERGE THIS PATTERN TO ABOVE! (CORRECT ORDER IS REQUIRED)
            lin_result.remove_and_subst(var2.clone(), var1.clone());
        },
        (_, LinExprIdx::RHS(_)) => {
            lin_result.remove_and_subst(var1.clone(), var2.clone());
        },
        (LinExprIdx::Cond(idx1), LinExprIdx::Cond(idx2)) => {
            // merge to smaller one
            let idx_val1 = *idx1.borrow();
            let idx_val2 = *idx2.borrow();
            if idx_val1 < idx_val2 {
                lin_result.remove_and_subst(var2.clone(), var1.clone());
            }
            else {
                lin_result.remove_and_subst(var1.clone(), var2.clone());
            }
        },
    }

}

fn handle_bool_expr(lin_result: LinResult, is_prim_true: bool, name: String, params: Vec<LinExprIdx>) -> Vec<LinResult> {
    let mut new_lin_result = lin_result;

    match name.as_str() {
        "u64_is_odd" => {
            assert!(params.len() == 1);
            let param_stmt = new_lin_result.get_mut(&params[0]).unwrap();
            match param_stmt {
                LinExpr::Var(ref mut conds) => {
                    if is_prim_true {
                        conds.push(vec![CondExpr::Expr { name: String::from("is_odd"), params: Vec::new() }])
                    }
                    else {
                        conds.push(vec![CondExpr::Expr { name: String::from("is_even"), params: Vec::new() }])
                    }
                },
                _ => (), // ignore
            }
        },
        "u64_is_zero" => {
            assert!(params.len() == 1);
            let param_stmt = new_lin_result.get_mut(&params[0]).unwrap();
            match param_stmt {
                LinExpr::Var(ref mut conds) => {
                    if is_prim_true {
                        *param_stmt = LinExpr::Const(0);
                    }
                    else {
                        conds.push(vec![CondExpr::Expr { name: String::from("nonzero"), params: Vec::new() }])
                    }
                },
                _ => (), // ignore
            }
        },
        "u64_lt" | "u8_lt" => {
            assert!(params.len() == 2);
            let mut new_conds_vec = Vec::new();
            convert_to_condexpr(&mut new_conds_vec, &new_lin_result, &params[1]);
            let param_stmt = new_lin_result.get_mut(&params[0]).unwrap();
            match param_stmt {
                LinExpr::Var(ref mut conds) => {
                    let new_params = vec![LinExprIdx::Cond(Rc::new(RefCell::new(new_conds_vec.len() - 1)))];
                    if is_prim_true {
                        new_conds_vec.push(CondExpr::Expr { name: String::from("lt"), params: new_params })
                    }
                    else {
                        new_conds_vec.push(CondExpr::Expr { name: String::from("ge"), params: new_params })
                    }
                    conds.push(new_conds_vec);
                },
                _ => (), // ignore
            }
        },
        "u64_le" | "u32_lteq" | "u8_lteq" => {
            assert!(params.len() == 2);
            let mut new_conds_vec = Vec::new();
            convert_to_condexpr(&mut new_conds_vec, &new_lin_result, &params[1]);
            let param_stmt = new_lin_result.get_mut(&params[0]).unwrap();
            match param_stmt {
                LinExpr::Var(ref mut conds) => {
                    let new_params = vec![LinExprIdx::Cond(Rc::new(RefCell::new(new_conds_vec.len() - 1)))];
                    if is_prim_true {
                        new_conds_vec.push(CondExpr::Expr { name: String::from("le"), params: new_params })
                    }
                    else {
                        new_conds_vec.push(CondExpr::Expr { name: String::from("gt"), params: new_params })
                    }
                    conds.push(new_conds_vec);
                },
                _ => (), // ignore
            }
        },
        "u64_eq" => {
            assert!(params.len() == 2);
            if is_prim_true { // only work for true
                return process_rule_cond_pair(new_lin_result, &params[0], &params[1]);
            }
        },
        _ => (), // ignore
    }
    vec![new_lin_result]
}

fn convert_to_condexpr(conds: &mut Vec<CondExpr>, lin_result: &LinResult, expr_idx: &LinExprIdx) {
    let stmt = lin_result.get(expr_idx).unwrap();
    match stmt {
        LinExpr::Var(_) | LinExpr::TypeVar(_) => conds.push(CondExpr::Var),
        LinExpr::Const(val) => conds.push(CondExpr::Const(*val)),
        LinExpr::ConstPrim(sym) => conds.push(CondExpr::ConstPrim(sym.clone())),
        LinExpr::Expr { name, params } => {
            let mut new_params = Vec::new();
            for param in params {
                match param {
                    LinExprIdx::LHS(_) |
                    LinExprIdx::RHS(_) => {
                        new_params.push(param.clone());
                    },
                    LinExprIdx::Cond(_) => {
                        convert_to_condexpr(conds, lin_result, param);
                        new_params.push(LinExprIdx::Cond(Rc::new(RefCell::new(conds.len() - 1))));
                    },
                }
            }
            conds.push(CondExpr::Expr { name: name.clone(), params: new_params })
        },
        LinExpr::Ident(ident_idx) => conds.push(CondExpr::Ident(ident_idx.clone())),
    }
}

fn add_expr_to_idx(lin_result: &mut LinResult, idx: &LinExprIdx, expr_idx: &LinExprIdx) -> LinExprIdx {
    let expr_stmt = lin_result.get(expr_idx).unwrap().clone();
    match expr_stmt {
        LinExpr::Expr { name, params } => {
            let new_params = params.iter().map(|x| add_expr_to_idx(lin_result, idx, x)).collect();
            let new_idx = idx.deep_clone();
            lin_result.insert_pair(new_idx.clone(), LinExpr::Expr { name, params: new_params });
            new_idx
        },
        LinExpr::Ident(ident_idx) => {
            match (idx, ident_idx.clone()) {
                (LinExprIdx::LHS(_), LinExprIdx::LHS(_)) |
                (LinExprIdx::RHS(_), LinExprIdx::LHS(_)) |
                (LinExprIdx::RHS(_), LinExprIdx::RHS(_)) => ident_idx,
                (LinExprIdx::LHS(_), LinExprIdx::RHS(_)) => {
                    let moved_idx = add_expr_to_idx(lin_result, idx, &ident_idx);
                    lin_result.remove_and_subst(ident_idx, moved_idx.clone());
                    moved_idx
                },
                (_, LinExprIdx::Cond(_)) => add_expr_to_idx(lin_result, idx, &ident_idx),
                _ => unreachable!(),
            }
        },
        _ => {
            let new_idx = idx.deep_clone();
            lin_result.insert_pair(new_idx.clone(), expr_stmt.clone());
            new_idx
        },
    }
}

fn copy_expr_to_var(lin_result: &mut LinResult, var_idx: &LinExprIdx, expr_idx: &LinExprIdx) {
    let expr_stmt = lin_result.get(expr_idx).unwrap().clone();
    match expr_stmt {
        LinExpr::Expr { name, params } => {
            let mut new_params = Vec::new();
            for param in params {
                match (var_idx, param.clone()) {
                    (LinExprIdx::LHS(_), LinExprIdx::LHS(_)) |
                    (LinExprIdx::RHS(_), LinExprIdx::LHS(_)) |
                    (LinExprIdx::RHS(_), LinExprIdx::RHS(_)) => {
                        new_params.push(param);
                    },
                    (LinExprIdx::LHS(_), LinExprIdx::RHS(_)) => {
                        let moved_idx = add_expr_to_idx(lin_result, var_idx, &param);
                        lin_result.remove_and_subst(param, moved_idx.clone());
                        new_params.push(moved_idx)
                    },
                    (_, LinExprIdx::Cond(_)) => {
                        new_params.push(add_expr_to_idx(lin_result, var_idx, &param));
                    },
                    _ => unreachable!(),
                }
            }
            *lin_result.get_mut(var_idx).unwrap() = LinExpr::Expr { name: name, params: new_params };
        },
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod test {
    use crate::{isle::ISLEParseOptions, isle_inl::process_internals, isle_lin::linearize_rules_opt};

    use super::*;

    #[test]
    fn test_process_conds() {
        let results = linearize_rules_opt(ISLEParseOptions::Lower);
        let inl_results: Vec<_> = results.into_iter().flat_map(process_internals).collect();
        println!("{:#?}", inl_results.into_iter().flat_map(process_conds).collect::<Vec<_>>());
    }
}