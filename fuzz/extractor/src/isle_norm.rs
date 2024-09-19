// Step 1.2: Normalize parsed ISLE rules

use std::collections::HashMap;

use cranelift_isle::ast::{Ident, Pattern, LetDef, Expr, IfLet, Rule};

use crate::{norm::{NormExpr, NormVar, NormConstraint, NormRule}, isle::{ISLEParseOptions, run_parse_opt}};

#[derive(Clone, Debug)]
struct NormEnv {
    vars: HashMap<String, NormVar>,
    bound_vars: HashMap<String, NormVar>,
    let_vars: HashMap<String, NormVar>,
    bound_exprs: Vec<NormExpr>
}

impl NormEnv {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
            bound_vars: HashMap::new(),
            let_vars: HashMap::new(),
            bound_exprs: Vec::new(),
        }
    }

    fn add_boundvar(&mut self, name: String, bound_expr: NormExpr) -> NormVar {
        assert!(!self.bound_vars.contains_key(&name));
        let bound_var_idx = self.bound_vars.len();

        self.bound_vars.insert(name, bound_var_idx);
        self.bound_exprs.push(bound_expr);

        bound_var_idx
    }

    fn add_letvar(&mut self, name: String, bound_expr: NormExpr) -> NormVar {
        let let_var_idx = self.let_vars.len();

        self.let_vars.insert(name, let_var_idx);
        self.bound_exprs.push(bound_expr);

        let_var_idx
    }

    fn get_bound_var_idx(&self, name: &String) -> Option<NormVar> {
        if self.let_vars.contains_key(name) {
            let let_var_idx = self.let_vars.get(name).unwrap();
            Some(*let_var_idx)
        }
        else if self.bound_vars.contains_key(name) {
            let bound_var_idx = self.bound_vars.get(name).unwrap();
            Some(*bound_var_idx)
        }
        else {
            None
        }
    }

    fn clear_letvars(&mut self) {
        self.let_vars.clear();
    }
}

fn process_sym(sym: &Ident) -> String {
    sym.0.clone()
}

fn process_pattern(pattern: &Pattern, env: &mut NormEnv) -> NormExpr {
    match pattern {
        Pattern::Var { var, pos: _ } => {
            let name = process_sym(var);

            match env.get_bound_var_idx(&name) {
                Some(idx) => { return NormExpr::BoundVar(idx); },
                None => (),
            }

            let var_idx: NormVar = match env.vars.get(&name) {
                Some(idx) => *idx,
                None => {
                    let new_var_idx = env.vars.len();
                    env.vars.insert(name, new_var_idx);
                    new_var_idx
                },
            };
            NormExpr::Var(var_idx)
        },
        Pattern::BindPattern { var, subpat, pos: _ } => {
            let name = process_sym(var);
            let bound_expr = process_pattern(subpat, env);
            let bound_var_idx = env.add_boundvar(name, bound_expr);
            NormExpr::BoundVar(bound_var_idx)
        },
        Pattern::ConstInt { val, pos: _ } => NormExpr::ConstInt(*val),
        Pattern::ConstPrim { val, pos: _ } => NormExpr::ConstPrim(process_sym(val)),
        Pattern::Term { sym, args, pos: _ } => {
            let name = process_sym(sym);
            let subexprs = args.iter().map(|x| process_pattern(x, env)).collect();
            
            NormExpr::Expr { name, subexprs }
        },
        Pattern::Wildcard { pos: _ } => NormExpr::Wildcard,
        Pattern::And { subpats, pos: _ } => {
            let subexprs = subpats.iter().map(|x| process_pattern(x, env)).collect();
            NormExpr::Expr { name: String::from("and"), subexprs }
        },
        Pattern::MacroArg { .. } => todo!(),
    }
}

fn process_letdef(letdef: &LetDef, env: &mut NormEnv) -> () {
    let name = process_sym(&letdef.var);
    let _type = process_sym(&letdef.ty);
    let val = process_expr(&letdef.val, env);

    let _let_var_idx = env.add_letvar(name, val);
}

fn process_expr(expr: &Expr, env: &mut NormEnv) -> NormExpr {
    match expr {
        Expr::Term { sym, args, pos: _ } => {
            let name = process_sym(sym);
            let subexprs = args.iter().map(|x| process_expr(x, env)).collect();
            NormExpr::Expr { name, subexprs }
        },
        Expr::Var { name: var, pos: _ } => {
            let name = process_sym(var);

            match env.get_bound_var_idx(&name) {
                Some(idx) => { return NormExpr::BoundVar(idx); },
                None => (),
            }

            let var_idx: NormVar = match env.vars.get(&name) {
                Some(idx) => *idx,
                None => {
                    let new_var_idx = env.vars.len();
                    env.vars.insert(name, new_var_idx);
                    new_var_idx
                },
            };
            NormExpr::Var(var_idx)
        },
        Expr::ConstInt { val, pos: _ } => NormExpr::ConstInt(*val),
        Expr::ConstPrim { val, pos: _ } => NormExpr::ConstPrim(process_sym(val)),
        Expr::Let { defs, body, pos: _ } => {
            for def in defs {
                process_letdef(def, env);
            }
            let result = process_expr(body, env);
            env.clear_letvars();
            result
        },
    }
}

fn process_iflet(iflet: &IfLet, env: &mut NormEnv) -> NormConstraint {
    let lhs = process_pattern(&iflet.pattern, env);
    let rhs = process_expr(&iflet.expr, env);
    NormConstraint { lhs, rhs }
}

fn process_rule(rule: &Rule, is_lower: bool) -> NormRule {
    let mut env = NormEnv::new();
    let lhs = process_pattern(&rule.pattern, &mut env);
    let rhs = process_expr(&rule.expr, &mut env);
    let constraints = rule.iflets.iter().map(|x| process_iflet(x, &mut env)).collect();
    NormRule {
        var_len: env.vars.len(),
        is_lower,
        lhs,
        rhs,
        constraints,
        bound_vars: env.bound_exprs,
    }
}

pub fn norm_rules_opt(opt: ISLEParseOptions) -> Vec<NormRule> {
    let parsed_result = run_parse_opt(opt).unwrap();
    let mut norm_rules = Vec::new();
    for def in parsed_result.defs {
        match def {
            cranelift_isle::ast::Def::Rule(rule) => {
                let norm_rule = process_rule(&rule, opt.is_lower());
                norm_rules.push(norm_rule);
            },
            _ => continue,
        }
    }
    norm_rules
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_process_rules() {
        println!("{:#?}", norm_rules_opt(ISLEParseOptions::Lower));
    }
}