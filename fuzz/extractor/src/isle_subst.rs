// Step 1.5: Substitute typed ISLE rules

use std::collections::HashMap;

use anyhow::{Context, Error};

use crate::norm::{CondExpr, LinExpr, LinExprIdx, LinResult, MatchStmt};

type SubstMap = HashMap<String, Vec<LinResult>>;

// Step 1: substitution of rules
pub fn subst_result(subst_map: &SubstMap, lin_result: LinResult, is_lower: bool) -> Result<Vec<LinResult>, Error> {
    // return Some with substituted results
    // return None when substitution failed
    fn subst_result_try(subst_map: &SubstMap, lin_result: &LinResult, cur_idx: LinExprIdx) -> Result<Option<Vec<LinResult>>, Error> {
        let stmt = lin_result.get(&cur_idx).context("Invalid cur_idx")?;
        match stmt {
            LinExpr::Expr { name, .. } => {
                let candidates = match subst_map.get(name) {
                    Some(x) => x,
                    None => {
                        return Ok(None);
                    }
                };
                
                let mut results = Vec::new();
                for candidate in candidates {
                    let mut dummy_match_stmts = vec![MatchStmt::None; lin_result.rule.lhs.len()];
                    let dummy_subst_stmts = vec![MatchStmt::None; candidate.rule.rhs.len()];
                    let subst_result = subst_apply_result(lin_result, candidate, &cur_idx, &mut dummy_match_stmts, &dummy_subst_stmts)?;
                    match subst_result {
                        Some(x) => results.push(x),
                        None => (),
                    }
                }
                if results.len() > 0 {
                    Ok(Some(results))
                }
                else {
                    Ok(None)
                }
            },
            _ => {
                Ok(None)
            }
        }
    }

    let mut result_vec = Vec::new(); // stores lin_results with complete substitution
    let mut pending_vec = vec![lin_result.deep_clone()]; // pending lin_results ready for substitution
    loop {
        if pending_vec.len() == 0 {
            break;
        }

        let temp_vec = pending_vec;
        pending_vec = Vec::new();

        for target_lin_result in temp_vec {
            let mut is_result_done = true;
            let idx_list = if is_lower {
                let mut idx_list = Vec::new();
                idx_list.append(&mut (0..target_lin_result.rule.lhs.len()).map(
                    |x| LinExprIdx::LHS(target_lin_result.rule.lhs.get_idx_ref(x).unwrap().clone())
                ).collect());
                idx_list.append(&mut (0..target_lin_result.cond_stmts.len()).map(
                    |x| LinExprIdx::Cond(target_lin_result.cond_stmts.get_idx_ref(x).unwrap().clone())
                ).collect());
                idx_list
            } else {
                target_lin_result.get_idx_list()
            };
            for cur_idx in idx_list {
                match subst_result_try(subst_map, &target_lin_result, cur_idx)? {
                    Some(mut subst_result) => {
                        pending_vec.append(&mut subst_result);
                        is_result_done = false;
                        break;
                    },
                    None => continue,
                }
            }

            if is_result_done {
                result_vec.push(target_lin_result);
            }
        }
    }
    Ok(result_vec)
}

// match lin_results and apply substitution at stmt_idx
// returns Ok(None) when match failed
// returns Ok(Some) with subst result and new stmt_idx
// also used in matching
pub fn subst_apply_result(
    lin_result: &LinResult, 
    subst: &LinResult, 
    stmt_idx: &LinExprIdx, 
    match_stmts: &mut Vec<MatchStmt>, // match stmts
    subst_stmts: &Vec<MatchStmt>, // subst_stmts
) -> Result<Option<LinResult>, Error> {
    assert!(match_stmts.len() == lin_result.rule.lhs.len());
    assert!(subst_stmts.len() == subst.rule.rhs.len());

    // Step 1: construct subst map (with matching)
    let mut subst_map = Vec::new();
    let subst_idx = LinExprIdx::LHS(subst.rule.lhs.get_idx_ref(subst.rule.lhs.len() - 1).unwrap().clone());

    let mut new_lin_result = lin_result.deep_clone();
    let new_stmt_idx = new_lin_result.get_idx(stmt_idx).unwrap();
    if !construct_subst_map(&mut subst_map, &mut new_lin_result, subst, &new_stmt_idx, &subst_idx)? {
        return Ok(None)
    }

    // Step 2: apply substitution map
    apply_subst_map(&subst_map, &mut new_lin_result, subst, &new_stmt_idx, &subst_idx, match_stmts, subst_stmts)?;

    // Step 3: remove dangling expressions
    new_lin_result.remove_dangling_expr(match_stmts);
    new_lin_result.remove_redundant_idents(match_stmts);
    
    Ok(Some(new_lin_result))
}

// check applicability of subst
// also used in matching
pub fn check_subst_applicable(
    lin_result: &LinResult, 
    subst: &LinResult, 
    stmt_idx: &LinExprIdx,
) -> bool {
    let mut subst_map = Vec::new();
    let subst_idx = LinExprIdx::LHS(subst.rule.lhs.get_idx_ref(subst.rule.lhs.len() - 1).unwrap().clone());

    let mut new_lin_result = lin_result.deep_clone();
    let new_stmt_idx = new_lin_result.get_idx(stmt_idx).unwrap();
    match construct_subst_map(&mut subst_map, &mut new_lin_result, subst, &new_stmt_idx, &subst_idx) {
        Ok(true) => true,
        _ => false,
    }
}

// constructs a substitution map with matching
// match direction: lin_result_what -> lin_result_with
fn construct_subst_map(
    expr_map: &mut Vec<(LinExprIdx, LinExprIdx)>, // expr_with -> expr_what (orders reversed)
    lin_result_what: &mut LinResult, // specialize type variables when needed
    lin_result_with: &LinResult,
    idx_what: &LinExprIdx,
    idx_with: &LinExprIdx
) -> Result<bool, Error> {
    let stmt_what = lin_result_what.get(idx_what).context("Invalid idx_what")?.clone();
    let stmt_with = lin_result_with.get(idx_with).context("Invalid idx_with")?.clone();
    match (stmt_what, stmt_with) {
        (_, LinExpr::Ident(_)) => unreachable!(),
        (LinExpr::Expr { name: name_what, params: params_what },
         LinExpr::Expr { name: name_with, params: params_with }) => {
            if name_what != name_with {
                return Ok(false);
            }
            if params_what.len() != params_with.len() {
                return Ok(false);
            }

            let param_result = params_what.iter().zip(params_with.iter()).map(
                |(inner_idx_what, inner_idx_with)| construct_subst_map(expr_map, lin_result_what, lin_result_with, inner_idx_what, inner_idx_with)
            ).collect::<Result<Vec<_>, _>>()?.into_iter().fold(true, |acc, x| acc && x);

            if param_result {
                match expr_map.iter().find(|(x, _)| x == idx_with) {
                    Some((_, y)) => { if y != idx_what { return Ok(false); } },
                    None => expr_map.push((idx_with.clone(), idx_what.clone())), // orders reversed (with -> what)
                };
            }
            Ok(param_result)
        },
        (LinExpr::TypeVar(_), LinExpr::Var(_)) |
        (LinExpr::Var(_), LinExpr::TypeVar(_)) => {
            Ok(false)
        },
        (LinExpr::TypeVar(types_what), LinExpr::TypeVar(types_with)) => {
            match expr_map.iter().find(|(x, _)| x == idx_with) {
                Some((_, y)) => { if y != idx_what { return Ok(false); } },
                None => expr_map.push((idx_with.clone(), idx_what.clone())), // orders reversed (with -> what)
            };
            let new_types = types_what.clone().into_iter().filter(|x| types_with.contains(x)).collect::<Vec<_>>();
            if new_types.len() == 0 {
                return Ok(false);
                // bail!("Type intersection non-existent in substitution");
            }

            let new_expr = LinExpr::TypeVar(new_types);
            lin_result_what.replace(idx_what, new_expr);

            Ok(true)
        }
        (_, LinExpr::Var(_)) |
        (LinExpr::Var(_), _) => { // Var in expr_what may come from wildcard
            match expr_map.iter().find(|(x, _)| x == idx_with) {
                Some((_, y)) => { if y != idx_what { return Ok(false); } },
                None => expr_map.push((idx_with.clone(), idx_what.clone())), // orders reversed (with -> what)
            };
            Ok(true)
        },
        (LinExpr::Const(val_what), LinExpr::Const(val_with)) => {
            if val_what == val_with {
                match expr_map.iter().find(|(x, _)| x == idx_with) {
                    Some((_, y)) => { if y != idx_what { return Ok(false); } },
                    None => expr_map.push((idx_with.clone(), idx_what.clone())), // orders reversed (with -> what)
                };
                Ok(true)
            }
            else {
                Ok(false)
            }
        },
        (LinExpr::ConstPrim(sym_what), LinExpr::ConstPrim(sym_with)) => {
            if sym_what == sym_with {
                match expr_map.iter().find(|(x, _)| x == idx_with) {
                    Some((_, y)) => { if y != idx_what { return Ok(false); } },
                    None => expr_map.push((idx_with.clone(), idx_what.clone())), // orders reversed (with -> what)
                };
                Ok(true)
            }
            else {
                Ok(false)
            }
        },
        (LinExpr::Ident(alias_idx), _) => {
            construct_subst_map(expr_map, lin_result_what, lin_result_with, &alias_idx, idx_with)
        },
        _ => Ok(false),
    }
}

fn apply_subst_map(
    expr_map: &Vec<(LinExprIdx, LinExprIdx)>, // expr_with -> expr_what (orders reversed)
    lin_result_what: &mut LinResult,
    lin_result_with: &LinResult,
    idx_what: &LinExprIdx,
    idx_with: &LinExprIdx,
    match_stmts: &mut Vec<MatchStmt>, // match stmts
    subst_stmts: &Vec<MatchStmt>, // subst_stmts
) -> Result<(), Error> {
    assert!(match_stmts.len() == lin_result_what.rule.lhs.len());
    assert!(subst_stmts.len() == lin_result_with.rule.rhs.len());
    
    // check validity of expr_map
    let (_, idx_what_find) = expr_map.iter().find(|(x, _)| x == idx_with).context("Invalid expr_map")?;
    assert!(idx_what == idx_what_find);

    // append RHS
    let idx_ref_what_rhs_list = (0..lin_result_with.rule.rhs.len()).into_iter().map(|_| idx_what.deep_clone()).collect::<Vec<_>>();
    for idx_with_rhs in (0..lin_result_with.rule.rhs.len()).rev() {
        let rhs_with_stmt = lin_result_with.rule.rhs.get(idx_with_rhs).unwrap();
        let mut new_expr = match rhs_with_stmt {
            LinExpr::Expr { name, params } => {
                let new_params = params.into_iter().map(
                    |x| apply_subst_map_idx(expr_map, &idx_ref_what_rhs_list, x)
                ).collect::<Result<Vec<_>, Error>>()?;
                LinExpr::Expr { name: name.clone(), params: new_params }
            },
            LinExpr::Ident(ident_idx) => {
                LinExpr::Ident(apply_subst_map_idx(expr_map, &idx_ref_what_rhs_list, ident_idx)?)
            },
            LinExpr::Var(conds) => {
                let new_conds = conds.into_iter().map(|cond_vec| {
                    cond_vec.into_iter().map(|x| {
                        match x {
                            CondExpr::Expr { name, params } => {
                                let new_params = params.into_iter().map(
                                    |x| apply_subst_map_idx(expr_map, &idx_ref_what_rhs_list, x)
                                ).collect::<Result<Vec<_>, Error>>()?;
                                Ok(CondExpr::Expr { name: name.clone(), params: new_params })
                            },
                            CondExpr::Ident(idx) => {
                                let new_idx = apply_subst_map_idx(expr_map, &idx_ref_what_rhs_list, idx)?;
                                Ok(CondExpr::Ident(new_idx))
                            },
                            _ => Ok(x.clone()),
                        }
                    }).collect::<Result<Vec<_>, Error>>()
                }).collect::<Result<Vec<_>, Error>>()?;
                LinExpr::Var(new_conds)
            },
            x => x.clone(),
        };

        if idx_with_rhs == lin_result_with.rule.rhs.len() - 1 {
            let stmt_what = lin_result_what.get_mut(idx_what).context("Invalid idx_what")?;
            let stmt_with = match (stmt_what, &mut new_expr) {
                // merge conds
                (LinExpr::Var(ref mut conds1), LinExpr::Var(ref mut conds2)) => {
                    conds1.append(conds2);
                    MatchStmt::Arg(conds1.clone())
                },
                _ => {
                    lin_result_what.replace(idx_what, new_expr);
                    subst_stmts[idx_with_rhs].clone()
                },
            };

            // replace match result
            match idx_what {
                LinExprIdx::LHS(idx) => {
                    match_stmts[*idx.borrow()] = stmt_with;
                },
                _ => (),
            }
        }
        else {
            let stmt_with = match (&new_expr, &subst_stmts[idx_with_rhs]) {
                (LinExpr::Var(conds1), MatchStmt::Arg(conds2)) => {
                    let mut new_conds = conds1.clone();
                    new_conds.extend(conds2.clone());
                    MatchStmt::Arg(new_conds)
                },
                _ => subst_stmts[idx_with_rhs].clone(),
            };

            // insert at first position, then the indices will be adjusted automatically
            lin_result_what.insert_pair(idx_ref_what_rhs_list[idx_with_rhs].clone(), new_expr);

            // insert match result
            match &idx_ref_what_rhs_list[idx_with_rhs] {
                LinExprIdx::LHS(idx) => {
                    match_stmts.insert(*idx.borrow(), stmt_with);
                },
                _ => (),
            }
        }
    }

    // append Cond
    let cond_shift_amt = lin_result_what.cond_stmts.len();
    for idx_with_cond in 0..lin_result_with.cond_stmts.len() {
        let cond_with_stmt = lin_result_with.cond_stmts.get(idx_with_cond).unwrap();
        match cond_with_stmt {
            LinExpr::Expr { name, params } => {
                assert!(lin_result_what.cond_stmts.len() == cond_shift_amt + idx_with_cond);
                let new_params = params.into_iter().map(|x| match x {
                    LinExprIdx::LHS(_) => {
                        let (_, idx_find) = expr_map.iter().find(|(y, _)| y == x).context("Invalid expr_map")?;
                        Ok(idx_find.clone())
                    },
                    LinExprIdx::RHS(idx) => Ok(idx_ref_what_rhs_list[*idx.borrow()].clone()),
                    LinExprIdx::Cond(idx) => {
                        Ok(LinExprIdx::Cond(lin_result_what.cond_stmts.get_idx_ref(*idx.borrow() + cond_shift_amt).unwrap().clone()))
                    },
                }).collect::<Result<Vec<_>, Error>>()?;
                let new_expr = LinExpr::Expr { name: name.clone(), params: new_params };
                lin_result_what.cond_stmts.push(new_expr);
            },
            LinExpr::Ident(idx) => {
                match idx {
                    LinExprIdx::LHS(_) => {
                        let (_, new_idx) = expr_map.iter().find(|(x, _)| x == idx).context("Invalid expr_map")?;
                        lin_result_what.cond_stmts.push(LinExpr::Ident(new_idx.clone()));
                    },
                    LinExprIdx::RHS(idx) => {
                        lin_result_what.cond_stmts.push(LinExpr::Ident(idx_ref_what_rhs_list[*idx.borrow()].clone()));
                    },
                    LinExprIdx::Cond(idx) => {
                        let new_idx = LinExprIdx::Cond(lin_result_what.cond_stmts.get_idx_ref(*idx.borrow() + cond_shift_amt).unwrap().clone());
                        lin_result_what.cond_stmts.push(LinExpr::Ident(new_idx));
                    },
                }
            },
            _ => {
                lin_result_what.cond_stmts.push(cond_with_stmt.clone());
            }
        }
    }

    for (fst, snd) in &lin_result_with.cond_pairs {
        let results = [fst, snd].into_iter().map(|x| match x {
            LinExprIdx::LHS(_) => {
                let (_, new_idx) = expr_map.iter().find(|(y, _)| y == x).context("Invalid expr_map")?;
                Ok(new_idx.clone())
            },
            LinExprIdx::RHS(idx) => {
                Ok(idx_ref_what_rhs_list[*idx.borrow()].clone())
            },
            LinExprIdx::Cond(idx) => {
                Ok(LinExprIdx::Cond(lin_result_what.cond_stmts.get_idx_ref(*idx.borrow() + cond_shift_amt).unwrap().clone()))
            },
        }).collect::<Result<Vec<_>, Error>>()?;
        lin_result_what.cond_pairs.push((results[0].clone(), results[1].clone())); // improve later...
    }

    Ok(())
}

fn apply_subst_map_idx(expr_map: &Vec<(LinExprIdx, LinExprIdx)>, idx_ref_what_rhs_list: &Vec<LinExprIdx>, target_idx: &LinExprIdx) 
    -> Result<LinExprIdx, Error> {
    match target_idx {
        LinExprIdx::LHS(_) => {
            let (_, idx_find) = expr_map.iter().find(|(x, _)| x == target_idx).context("Invalid expr_map")?;
            Ok(idx_find.clone())
        },
        LinExprIdx::RHS(idx) => Ok(idx_ref_what_rhs_list[*idx.borrow()].clone()),
        LinExprIdx::Cond(_) => Ok(target_idx.clone()),
    }
}

#[cfg(test)]
mod test {
    use crate::{isle::ISLEParseOptions, isle_type::type_rules_opt};

    use super::*;

    fn prepare_subst_map(parse_option: ISLEParseOptions) -> HashMap<String, Vec<LinResult>> {
        let clir_results = type_rules_opt(parse_option);
        let mut clir_simplify_results = Vec::new();
        let mut clir_lower_results = Vec::new();
        let mut subst_map: HashMap<String, Vec<LinResult>> = HashMap::new();
        for lin_result in clir_results {
            let lhs_stmt = lin_result.rule.lhs.get(lin_result.rule.lhs.len() - 1).unwrap();
            match lhs_stmt {
                LinExpr::Expr { name, params} => {
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
                        subst_map.entry(name.clone()).or_default().push(lin_result);
                    }
                },
                _ => unreachable!(),
            }
        }

        subst_map
    }

    #[test]
    fn test_subst_results() {
        let parse_option = ISLEParseOptions::Lower;
        let subst_map = prepare_subst_map(parse_option);
        let results = type_rules_opt(parse_option);
        for lin_result in results {
            let cur_result = subst_result(&subst_map, lin_result, true);
            println!("{:#?}", cur_result);
        }
    }

    #[test]
    fn test_subst_result_one() {
        let parse_option = ISLEParseOptions::TestOpt;
        let subst_map = prepare_subst_map(parse_option);
        let results = type_rules_opt(parse_option);
        println!("{:#?}", subst_result(&subst_map, results.last().unwrap().clone(), false));
    }
}