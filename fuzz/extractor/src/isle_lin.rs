// Step 1.3: Linearize normalized ISLE rules

use std::collections::HashMap;

use crate::{norm::{get_all_types, LinExpr, LinExprIdx, LinResult, LinRule, LinType, LinVec, NormExpr, NormRule, NormVar}, isle::ISLEParseOptions, isle_norm::norm_rules_opt};

#[derive(Clone, Debug)]
struct LinEnv {
    lhs: LinVec<LinExpr>,
    rhs: LinVec<LinExpr>,
    cond: LinVec<LinExpr>,
    var_map: HashMap<NormVar, LinExprIdx>,
    bound_var_map: HashMap<NormVar, LinExprIdx>,
    bound_vars: Vec<NormExpr>,
}

impl LinEnv {
    fn new(bound_vars: Vec<NormExpr>) -> Self {
        Self {
            lhs: LinVec::new(),
            rhs: LinVec::new(),
            cond: LinVec::new(),
            var_map: HashMap::new(),
            bound_var_map: HashMap::new(),
            bound_vars,
        }
    }
}

fn linearize_expr(env: &mut LinEnv, expr: NormExpr, lin_type: LinType, is_lower: bool) -> LinExprIdx {
    match expr {
        NormExpr::Var(varid) => {
            if env.var_map.contains_key(&varid) {
                env.var_map.get(&varid).unwrap().clone()
            }
            else {
                let new_expr = LinExpr::Var(Vec::new());
                match lin_type {
                    LinType::LHS => {
                        let idx = LinExprIdx::LHS(env.lhs.push(new_expr));
                        env.var_map.insert(varid, idx.clone());
                        idx
                    },
                    LinType::RHS => {
                        let idx = LinExprIdx::RHS(env.rhs.push(new_expr));
                        env.var_map.insert(varid, idx.clone());
                        idx
                    },
                    LinType::Cond => {
                        let idx = LinExprIdx::Cond(env.cond.push(new_expr));
                        env.var_map.insert(varid, idx.clone());
                        idx
                    },
                }
            }
        },
        NormExpr::Expr { name, subexprs } => {
            let mut params = Vec::new();
            if is_lower && name != "lower" {
                // add typevar to the first of the expression
                let typevar_param = match lin_type {
                    LinType::LHS => LinExprIdx::LHS(env.lhs.push(LinExpr::TypeVar(get_all_types().to_vec()))),
                    LinType::RHS => LinExprIdx::RHS(env.rhs.push(LinExpr::TypeVar(get_all_types().to_vec()))),
                    LinType::Cond => LinExprIdx::Cond(env.cond.push(LinExpr::TypeVar(get_all_types().to_vec()))),
                };
                params.push(typevar_param);
            }
            params.append(&mut subexprs.into_iter().map(|x| linearize_expr(env, x, lin_type.clone(), is_lower)).collect());

            // change first var of expr to typevar
            if !is_lower && params.len() > 0 {
                match &params[0] {
                    LinExprIdx::LHS(idx_cell) => {
                        let idx = *idx_cell.borrow();
                        match env.lhs.get(idx) {
                            Some(LinExpr::Var(_)) => {
                                *env.lhs.get_mut(idx).unwrap() = LinExpr::TypeVar(get_all_types().to_vec());
                            },
                            _ => (),
                        }
                    },
                    LinExprIdx::RHS(idx_cell) => {
                        let idx = *idx_cell.borrow();
                        match env.rhs.get(idx) {
                            Some(LinExpr::Var(_)) => {
                                *env.rhs.get_mut(idx).unwrap() = LinExpr::TypeVar(get_all_types().to_vec());
                            },
                            _ => (),
                        }
                    },
                    LinExprIdx::Cond(idx_cell) => {
                        let idx = *idx_cell.borrow();
                        match env.cond.get(idx) {
                            Some(LinExpr::Var(_)) => {
                                *env.cond.get_mut(idx).unwrap() = LinExpr::TypeVar(get_all_types().to_vec());
                            },
                            _ => (),
                        }
                    },
                }
            }

            let new_expr = LinExpr::Expr { name, params };
            match lin_type {
                LinType::LHS => LinExprIdx::LHS(env.lhs.push(new_expr)),
                LinType::RHS => LinExprIdx::RHS(env.rhs.push(new_expr)),
                LinType::Cond => LinExprIdx::Cond(env.cond.push(new_expr)),
            }
        },
        NormExpr::Wildcard => {
            let new_expr = LinExpr::Var(Vec::new());
            match lin_type {
                LinType::LHS => LinExprIdx::LHS(env.lhs.push(new_expr)),
                LinType::RHS => LinExprIdx::RHS(env.rhs.push(new_expr)),
                LinType::Cond => LinExprIdx::Cond(env.cond.push(new_expr)),
            }
        },
        NormExpr::ConstInt(val) => {
            let new_expr = LinExpr::Const(val);
            match lin_type {
                LinType::LHS => LinExprIdx::LHS(env.lhs.push(new_expr)),
                LinType::RHS => LinExprIdx::RHS(env.rhs.push(new_expr)),
                LinType::Cond => LinExprIdx::Cond(env.cond.push(new_expr)),
            }
        },
        NormExpr::ConstPrim(sym) => {
            let new_expr = LinExpr::ConstPrim(sym);
            match lin_type {
                LinType::LHS => LinExprIdx::LHS(env.lhs.push(new_expr)),
                LinType::RHS => LinExprIdx::RHS(env.rhs.push(new_expr)),
                LinType::Cond => LinExprIdx::Cond(env.cond.push(new_expr)),
            }
        },
        NormExpr::BoundVar(varid) => {
            if env.bound_var_map.contains_key(&varid) {
                env.bound_var_map.get(&varid).unwrap().clone()
            }
            else {
                let new_idx = linearize_expr(env, env.bound_vars[varid].clone(), lin_type, is_lower);
                env.bound_var_map.insert(varid, new_idx.clone());
                new_idx
            }
        },
    }
}

// returns linearized rule, cond stmts, cond indices
fn linearize_rule(rule: NormRule) -> LinResult {
    let mut env = LinEnv::new(rule.bound_vars);
    let _ = linearize_expr(&mut env, rule.lhs, LinType::LHS, rule.is_lower);
    let rhs_idx = linearize_expr(&mut env, rule.rhs, LinType::RHS, rule.is_lower);
    if env.rhs.len() == 0 {
        // need ident
        env.rhs.push(LinExpr::Ident(rhs_idx));
    }

    let mut conds = Vec::new();
    for rule_cond in rule.constraints {
        let lhs = linearize_expr(&mut env, rule_cond.lhs, LinType::Cond, rule.is_lower);
        let rhs = linearize_expr(&mut env, rule_cond.rhs, LinType::Cond, rule.is_lower);
        conds.push((lhs, rhs));
    }
    
    LinResult {
        rule: LinRule::new(env.lhs, env.rhs, rule.is_lower),
        cond_stmts: env.cond,
        cond_pairs: conds,
    }
}

pub fn linearize_rules_opt(opt: ISLEParseOptions) -> Vec<LinResult> {
    let norm_rules = norm_rules_opt(opt);
    norm_rules.into_iter().map(linearize_rule).collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_linearize_rules() {
        let rules = linearize_rules_opt(ISLEParseOptions::TestOpt);
        println!("{:#?}", rules);
    }
}