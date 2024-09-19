// Step 2.2: Convert (normalize) IRData to UnifiedExpr

use wasm_ast::Instruction;

use crate::{wasm_map::IRData, norm::{UnifiedExprIdx, LinVec, MatchResult, MatchStmt, UnifiedRule, UnifiedStmt}};

// returns UnifiedRule
// e.g., bxor: ty x y bxor -> bxor
fn irdata_to_unifiedexpr(irdata: &IRData) -> Option<UnifiedRule> {
    if irdata.ret_type.is_none() {
        return None // WARNING: may need to change this if no-return is supported
    }

    let name = irdata.opcode.to_string();
    let mut lhs_stmts = LinVec::new();
    lhs_stmts.push(UnifiedStmt::TypeVar(vec![irdata.ret_type?]));

    // IntCC or FloatCC -> second argument is CC
    let intcc_stmt = match irdata.intcc {
        Some(intcc) => {
            let cc_stmt = UnifiedStmt::Expr { name: format!("IntCC.{:?}", intcc), params: Vec::new() };
            lhs_stmts.push(cc_stmt.clone());
            Some(cc_stmt)
        },
        None => None,
    };
    let floatcc_stmt = match irdata.floatcc {
        Some(floatcc) => {
            let cc_stmt = UnifiedStmt::Expr { name: format!("FloatCC.{:?}", floatcc), params: Vec::new() };
            lhs_stmts.push(cc_stmt.clone());
            Some(cc_stmt)
        },
        None => None,
    };

    // operand takes a variable
    for _ in 0..((irdata.operand_type.is_some() as usize) + irdata.param_types.len()) {
        lhs_stmts.push(UnifiedStmt::Var(Vec::new()));
    }

    let mut params = (0..lhs_stmts.len()).map(|x| UnifiedExprIdx::LHS(lhs_stmts.get_idx_ref(x).unwrap().clone())).collect::<Vec<_>>();
    lhs_stmts.push(UnifiedStmt::Expr { name: name.clone(), params: params.clone() });
    
    let mut rhs_stmts = LinVec::new();
    match (intcc_stmt, floatcc_stmt) {
        (Some(x @ UnifiedStmt::Expr { .. }), None) |
        (None, Some(x @ UnifiedStmt::Expr { .. })) => {
            rhs_stmts.push(x);
            params[1] = UnifiedExprIdx::RHS(rhs_stmts.get_idx_ref(0).unwrap().clone());
            rhs_stmts.push(UnifiedStmt::Expr { name, params });
        },
        (None, None) => {
            rhs_stmts.push(UnifiedStmt::Expr { name, params });
        },
        _ => unreachable!(),
    }

    Some(UnifiedRule::new(lhs_stmts, rhs_stmts, false))
}

pub fn wasm_pair_to_matched_result(irdata: &IRData, instrs: &Vec<Instruction>) -> Option<MatchResult> {
    let rule = irdata_to_unifiedexpr(irdata)?;
    let match_stmts = if rule.rhs.len() == 1 {
        vec![MatchStmt::Expr { data: irdata.clone(), instrs: instrs.clone() }]
    } else {
        assert!(rule.rhs.len() == 2);
        vec![MatchStmt::Nil, MatchStmt::Expr { data: irdata.clone(), instrs: instrs.clone() }]
    };
    let matched_result = MatchResult::new_rev(rule, match_stmts);
    Some(matched_result)
}

#[cfg(test)]
mod test {
    use crate::wasm_map::get_clir_wasm_map;

    use super::*;
    
    #[test]
    fn test_irdata_to_unifiedexpr() {
        let map = get_clir_wasm_map();
        for irdata in map.keys() {
            println!("{}", "=".repeat(30));
            println!("{:#?}", irdata);
            println!("{:#?}", irdata_to_unifiedexpr(irdata));
        }
    }

    #[test]
    fn test_wasm_pair_to_matched_result() {
        let map = get_clir_wasm_map();
        for (irdata, instrs) in map {
            println!("{}", "=".repeat(30));
            println!("{:#?}", wasm_pair_to_matched_result(&irdata, &instrs));
        }
    }
}