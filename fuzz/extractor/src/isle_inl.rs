// Step 1.4.1: Process compiler internals of linearized ISLE rules

use std::collections::{HashMap, HashSet};

use cranelift_codegen::ir::{dynamic_to_fixed, types, Type};

use crate::norm::{get_all_types, get_imm128_types, get_imm32_types, get_imm64_types, get_types_intersection, LinExpr, LinExprIdx, LinResult, MatchStmt};

// Special names for custom identifiers
const VP_NAME: &str = "VP"; // value passing expression

// expression names that needs transformation of typevars back to vars
const TYPEVAR_REVERT_LIST: &'static [&'static str] = &[
    "i64_neg",
    "u64_add",
    "u64_sub",
    "u64_mul",
    "u64_sdiv",
    "u64_udiv",
    "u64_and",
    "u64_or",
    "u64_xor",
    "u64_shl",
    "u64_not",
    "u64_eq",
    "u64_le",
    "u64_lt",
    "u64_is_zero",
    "u64_is_odd",
    "u64_from_bool",
    "trap_code_division_by_zero",
    "trap_code_integer_overflow",
    "trap_code_bad_conversion_to_integer",
    "u32_add",
    "u32_sub",
    "u32_and",
    "s32_add_falliable",
    "u32_lteq",
    "u8_lteq",
    "u8_lt",
    "u8_and",
    "u8_shl",
    "u8_shr",
    "ty_equal", // ignored
    "range", // ignored
    "range_view", // ignored
    "mem_flags_trusted",
    "intcc_unsigned",
    "signed_cond_code", // ignored
    "intcc_reverse", // ignored
    "intcc_inverse", // ignored
    "floatcc_reverse", // ignored
    "floatcc_inverse", // ignored
    "floatcc_unordered", // ignored
    "imm12_and",
    "imm12_const_add",
    "offset32_add",
    "uimm32shifted",
    "uimm16shifted",
    "i64_nonequal",
];

pub fn process_internals(lin_result: LinResult) -> Vec<LinResult> {
    let mut results = vec![lin_result.deep_clone()];
    let mut will_break = false;
    while !will_break {
        will_break = true;
        let results_to_process = results;
        results = Vec::new();

        // Step 1: Expression rewriting with simple heuristics
        for result_to_process in results_to_process {
            match process_internals_all(&result_to_process) {
                Some(mut x) => {
                    will_break = false;
                    results.append(&mut x);
                },
                None => results.push(result_to_process),
            }
        }

        // Step 2: Remove all simple value passing statements
        results = results.into_iter().map(remove_value_passes).collect();

        // Step 3: Remove all dangling exprs
        for result in results.iter_mut() {
            let mut dummy_match_stmts = vec![MatchStmt::None; result.rule.lhs.len()];
            result.remove_dangling_expr(&mut dummy_match_stmts);
        }
    }
    results
}

fn remove_value_passes(lin_result: LinResult) -> LinResult {
    let mut result = lin_result;
    let idx_list = result.get_idx_list();

    // search how to remove and substitute
    let mut subst_list = Vec::new();
    for idx in idx_list {
        let stmt = result.get(&idx).unwrap();
        match stmt {
            LinExpr::Expr { name, params } if name == VP_NAME => {
                assert!(params.len() == 1);
                subst_list.push((idx, params[0].clone())); // clone only reference
            },
            _ => ()
        }
    }

    // perform substitution
    for idx in 0..subst_list.len() {
        let (target_idx, subst_idx) = subst_list[idx].clone();
        subst_list = subst_list.into_iter().map(|(x, y)| {
            let new_x = if x == target_idx { subst_idx.clone() } else { x };
            let new_y = if y == target_idx { subst_idx.clone() } else { y };
            (new_x, new_y)
        }).collect();
        result.remove_and_subst(target_idx.clone(), subst_idx.clone());
    }

    result
}

fn process_internals_all(lin_result: &LinResult) -> Option<Vec<LinResult>> {
    let mut new_results = vec![lin_result.deep_clone()];
    let mut is_some = false;

    let mut idx_list = Vec::new();
    idx_list.append(&mut (0..lin_result.rule.lhs.len()).map(
        |x| LinExprIdx::LHS(lin_result.rule.lhs.get_idx_ref(x).unwrap().clone())
    ).collect());
    idx_list.append(&mut (0..lin_result.rule.rhs.len()).map(
        |x| LinExprIdx::RHS(lin_result.rule.rhs.get_idx_ref(x).unwrap().clone())
    ).collect());
    idx_list.append(&mut (0..lin_result.cond_stmts.len()).map(
        |x| LinExprIdx::Cond(lin_result.cond_stmts.get_idx_ref(x).unwrap().clone())
    ).collect());

    for idx in idx_list {
        let mut tmp_rules = Vec::new();
        for new_result in new_results {
            match process_internals_one(&new_result, &idx) {
                Some(mut x) => {
                    is_some = true;
                    tmp_rules.append(&mut x)
                },
                None => tmp_rules.push(new_result),
            };
        }
        new_results = tmp_rules;
    }

    if is_some {
        Some(new_results)
    }
    else {
        None
    }
}

fn process_internals_one(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_constprim_type(lin_result, cur_idx)
        .or_else(|| inl_fits_in_64(lin_result, cur_idx))
        .or_else(|| inl_fits_in_32(lin_result, cur_idx))
        .or_else(|| inl_fits_in_16(lin_result, cur_idx))
        .or_else(|| inl_lane_fits_in_32(lin_result, cur_idx))
        .or_else(|| inl_ty_int_ref_scalar_64(lin_result, cur_idx))
        .or_else(|| inl_ty_32(lin_result, cur_idx))
        .or_else(|| inl_ty_64(lin_result, cur_idx))
        .or_else(|| inl_ty_32_or_64(lin_result, cur_idx))
        .or_else(|| inl_ty_8_or_16(lin_result, cur_idx))
        .or_else(|| inl_int_fits_in_32(lin_result, cur_idx))
        .or_else(|| inl_ty_int_ref_64(lin_result, cur_idx))
        .or_else(|| inl_ty_int_ref_16_to_64(lin_result, cur_idx))
        .or_else(|| inl_ty_int(lin_result, cur_idx))
        .or_else(|| inl_ty_scalar(lin_result, cur_idx))
        .or_else(|| inl_ty_scalar_float(lin_result, cur_idx))
        .or_else(|| inl_ty_float_or_vec(lin_result, cur_idx))
        .or_else(|| inl_ty_vector_float(lin_result, cur_idx))
        .or_else(|| inl_ty_vector_not_float(lin_result, cur_idx))
        .or_else(|| inl_ty_vec64(lin_result, cur_idx))
        .or_else(|| inl_ty_vec128(lin_result, cur_idx))
        .or_else(|| inl_ty_dyn_vec64(lin_result, cur_idx))
        .or_else(|| inl_ty_dyn_vec128(lin_result, cur_idx))
        .or_else(|| inl_ty_vec64_int(lin_result, cur_idx))
        .or_else(|| inl_ty_vec128_int(lin_result, cur_idx))
        .or_else(|| inl_ty_addr64(lin_result, cur_idx))
        .or_else(|| inl_ty_dyn64_int(lin_result, cur_idx))
        .or_else(|| inl_ty_dyn128_int(lin_result, cur_idx))
        .or_else(|| inl_not_vec32x2(lin_result, cur_idx))
        .or_else(|| inl_not_i64x2(lin_result, cur_idx))
        .or_else(|| inl_lane_type(lin_result, cur_idx))
        .or_else(|| inl_ty_half_lanes(lin_result, cur_idx))
        .or_else(|| inl_ty_half_width(lin_result, cur_idx))
        .or_else(|| inl_u64_from_imm64(lin_result, cur_idx))
        .or_else(|| inl_u128_from_immediate(lin_result, cur_idx))
        .or_else(|| inl_imm64(lin_result, cur_idx))
        .or_else(|| inl_imm64_masked(lin_result, cur_idx))
        .or_else(|| inl_simm32(lin_result, cur_idx))
        .or_else(|| inl_simm32_from_value(lin_result, cur_idx))
        .or_else(|| inl_simm32_from_imm64(lin_result, cur_idx))
        .or_else(|| inl_uimm8(lin_result, cur_idx))
        .or_else(|| inl_offset32(lin_result, cur_idx))
        .or_else(|| inl_offset32_to_u32(lin_result, cur_idx))
        .or_else(|| inl_u32_to_offset32(lin_result, cur_idx))
        .or_else(|| inl_u32_from_ieee32(lin_result, cur_idx))
        .or_else(|| inl_u64_from_ieee64(lin_result, cur_idx))
        .or_else(|| inl_emit_u64_le_const(lin_result, cur_idx))
        .or_else(|| inl_emit_u128_le_const(lin_result, cur_idx))
        .or_else(|| inl_multi_lane(lin_result, cur_idx))
        .or_else(|| inl_dynamic_lane(lin_result, cur_idx))
        .or_else(|| inl_i64_sextend_imm64(lin_result, cur_idx))
        .or_else(|| inl_u64_uextend_imm64(lin_result, cur_idx))
        .or_else(|| inl_subsume(lin_result, cur_idx))
        .or_else(|| inl_remat(lin_result, cur_idx))
        .or_else(|| inl_splat64(lin_result, cur_idx))
        .or_else(|| inl_ty_as_ty(lin_result, cur_idx))
        .or_else(|| inl_ty_from_ty_identity(lin_result, cur_idx))
        .or_else(|| inl_shift_amount_masked(lin_result, cur_idx))
        .or_else(|| inl_imm12_const(lin_result, cur_idx))
        .or_else(|| inl_offset32_imm(lin_result, cur_idx))
        .or_else(|| inl_u64_truncate_to_u32(lin_result, cur_idx))
        .or_else(|| inl_shift_masked_imm(lin_result, cur_idx))
        .or_else(|| inl_value_type(lin_result, cur_idx))
        .or_else(|| inl_imm64_power_of_two(lin_result, cur_idx))
        .or_else(|| inl_u64_nonzero(lin_result, cur_idx))
        .or_else(|| inl_nonzero_u64_fits_in_u32(lin_result, cur_idx))
        .or_else(|| inl_vconst_all_ones_or_all_zeros(lin_result, cur_idx))
        .or_else(|| inl_pshufd_lhs_imm(lin_result, cur_idx))
        .or_else(|| inl_pshufd_rhs_imm(lin_result, cur_idx))
        .or_else(|| inl_shufps_imm(lin_result, cur_idx))
        .or_else(|| inl_shufps_rev_imm(lin_result, cur_idx))
        .or_else(|| inl_pshuflw_lhs_imm(lin_result, cur_idx))
        .or_else(|| inl_pshuflw_rhs_imm(lin_result, cur_idx))
        .or_else(|| inl_pshufhw_lhs_imm(lin_result, cur_idx))
        .or_else(|| inl_pshufhw_rhs_imm(lin_result, cur_idx))
        .or_else(|| inl_palignr_imm_from_immediate(lin_result, cur_idx))
        .or_else(|| inl_pblendw_imm(lin_result, cur_idx))
        .or_else(|| inl_u32_nonnegative(lin_result, cur_idx))
        .or_else(|| inl_sse_interps_lane_imm(lin_result, cur_idx))
        .or_else(|| inl_u64_from_inverted_value(lin_result, cur_idx))
        .or_else(|| inl_uimm16shifted_from_inverted_value(lin_result, cur_idx))
        .or_else(|| inl_uimm32shifted_from_inverted_value(lin_result, cur_idx))
        .or_else(|| inl_i16_from_swapped_value(lin_result, cur_idx))
        .or_else(|| inl_i64_from_negated_value(lin_result, cur_idx))
        .or_else(|| inl_i32_from_negated_value(lin_result, cur_idx))
        .or_else(|| inl_i16_from_negated_value(lin_result, cur_idx))
        .or_else(|| inl_len_minus_one(lin_result, cur_idx))
        .or_else(|| inl_shuffle64_from_imm(lin_result, cur_idx))
        .or_else(|| inl_shuffle32_from_imm(lin_result, cur_idx))
        .or_else(|| inl_shuffle16_from_imm(lin_result, cur_idx))
        .or_else(|| inl_shuffle_dup8_from_imm(lin_result, cur_idx))
        .or_else(|| inl_shuffle_dup16_from_imm(lin_result, cur_idx))
        .or_else(|| inl_shuffle_dup32_from_imm(lin_result, cur_idx))
        .or_else(|| inl_shuffle_dup64_from_imm(lin_result, cur_idx))
        .or_else(|| inl_u64_low32_bits_unset(lin_result, cur_idx))
        .or_else(|| inl_u128_replicated_u64(lin_result, cur_idx))
        .or_else(|| inl_u64_replicated_u32(lin_result, cur_idx))
        .or_else(|| inl_u32_replicated_u16(lin_result, cur_idx))
        .or_else(|| inl_u16_replicated_u8(lin_result, cur_idx))
        .or_else(|| inl_neg_imm12(lin_result, cur_idx))
        .or_else(|| inl_safe_divisor_from_imm64(lin_result, cur_idx))
        .or_else(|| inl_has_type(lin_result, cur_idx))
        .or_else(|| inl_ty_bits(lin_result, cur_idx))
        .or_else(|| inl_ty_bytes(lin_result, cur_idx))
        .or_else(|| inl_ty_mask(lin_result, cur_idx))
        .or_else(|| inl_ty_lane_mask(lin_result, cur_idx))
        .or_else(|| inl_ty_lane_count(lin_result, cur_idx))
        .or_else(|| inl_ty_umin(lin_result, cur_idx))
        .or_else(|| inl_ty_umax(lin_result, cur_idx))
        .or_else(|| inl_ty_smin(lin_result, cur_idx))
        .or_else(|| inl_ty_smax(lin_result, cur_idx))
        .or_else(|| inl_shift_mask(lin_result, cur_idx))
        .or_else(|| inl_zero_offset(lin_result, cur_idx))
        .or_else(|| inl_fcvt_to_uint_ub32(lin_result, cur_idx))
        .or_else(|| inl_fcvt_to_uint_lb32(lin_result, cur_idx))
        .or_else(|| inl_fcvt_to_uint_ub64(lin_result, cur_idx))
        .or_else(|| inl_fcvt_to_uint_lb64(lin_result, cur_idx))
        .or_else(|| inl_fcvt_to_sint_ub32(lin_result, cur_idx))
        .or_else(|| inl_fcvt_to_sint_lb32(lin_result, cur_idx))
        .or_else(|| inl_fcvt_to_sint_ub64(lin_result, cur_idx))
        .or_else(|| inl_fcvt_to_sint_lb64(lin_result, cur_idx))
        .or_else(|| inl_u64_from_iconst(lin_result, cur_idx))
        .or_else(|| inl_revert_typevar(lin_result, cur_idx))
}

// $XXX: ConstPrim to types
// e.g., $I32 -> TypeVar([I32])
fn inl_constprim_type(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::ConstPrim(sym) => {
            for ty in get_all_types() {
                if ty.to_string() == sym.to_lowercase() {
                    let mut new_lin_result = lin_result.deep_clone();
                    *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(vec![ty]);
                    return Some(vec![new_lin_result]);
                }
            }
            None
        },
        _ => None
    }
}

// fits_in_64: specialize typevar
fn inl_fits_in_64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(
        |x| x.bits() <= 64 && !x.is_dynamic_vector()
    ).collect();
    inl_ty_intersect("fits_in_64", &intersect_types, lin_result, cur_idx)
}

// fits_in_32: specialize typevar
fn inl_fits_in_32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(
        |x| x.bits() <= 32 && !x.is_dynamic_vector()
    ).collect();
    inl_ty_intersect("fits_in_32", &intersect_types, lin_result, cur_idx)
}

// fits_in_16: specialize typevar
fn inl_fits_in_16(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(
        |x| x.bits() <= 16 && !x.is_dynamic_vector()
    ).collect();
    inl_ty_intersect("fits_in_16", &intersect_types, lin_result, cur_idx)
}

// lane_fits_in_32: specialize typevar
fn inl_lane_fits_in_32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| 
        if !ty.is_vector() && !ty.is_dynamic_vector() {
            false
        } else if ty.lane_type().bits() <= 32 {
            true
        } else {
            false
        }
    ).collect();
    inl_ty_intersect("lane_fits_in_32", &intersect_types, lin_result, cur_idx)
}

// ty_int_ref_scalar_64: specialize typevar
fn inl_ty_int_ref_scalar_64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| 
        if ty.bits() <= 64 && !ty.is_float() && !ty.is_vector() {
            true
        } else {
            false
        }
    ).collect();
    inl_ty_intersect("ty_int_ref_scalar_64", &intersect_types, lin_result, cur_idx)
        .or_else(|| inl_ty_intersect("ty_int_ref_scalar_64_extract", &intersect_types, lin_result, cur_idx))
}

// ty_32: specialize typevar
fn inl_ty_32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| ty.bits() == 32).collect();
    inl_ty_intersect("ty_32", &intersect_types, lin_result, cur_idx)
}

// ty_64: specialize typevar
fn inl_ty_64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| ty.bits() == 64).collect();
    inl_ty_intersect("ty_64", &intersect_types, lin_result, cur_idx)
}

// ty_32_or_64: specialize typevar
fn inl_ty_32_or_64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| ty.bits() == 32 || ty.bits() == 64).collect();
    inl_ty_intersect("ty_32_or_64", &intersect_types, lin_result, cur_idx)
}

// ty_8_or_16: specialize typevar
fn inl_ty_8_or_16(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| ty.bits() == 8 || ty.bits() == 16).collect();
    inl_ty_intersect("ty_8_or_16", &intersect_types, lin_result, cur_idx)
}

// int_fits_in_32: specialize typevar
fn inl_int_fits_in_32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = vec![types::I8, types::I16, types::I32];
    inl_ty_intersect("int_fits_in_32", &intersect_types, lin_result, cur_idx)
}

// ty_int_ref_64: specialize typevar
fn inl_ty_int_ref_64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = vec![types::I64, types::R64];
    inl_ty_intersect("ty_int_ref_64", &intersect_types, lin_result, cur_idx)
}

// ty_int_ref_16_to_64: specialize typevar
fn inl_ty_int_ref_16_to_64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = vec![types::I16, types::I32, types::I64, types::R64];
    inl_ty_intersect("ty_int_ref_16_to_64", &intersect_types, lin_result, cur_idx)
}

// ty_int: specialize typevar
fn inl_ty_int(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| ty.is_int()).collect();
    inl_ty_intersect("ty_int", &intersect_types, lin_result, cur_idx)
}

// ty_scalar: specialize typevar
fn inl_ty_scalar(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| ty.lane_count() == 1).collect();
    inl_ty_intersect("ty_scalar", &intersect_types, lin_result, cur_idx)
}

// ty_scalar_float: specialize typevar
fn inl_ty_scalar_float(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = vec![types::F32, types::F64];
    inl_ty_intersect("ty_scalar_float", &intersect_types, lin_result, cur_idx)
}

// ty_float_or_vec: specialize typevar
fn inl_ty_float_or_vec(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        match ty {
            &types::F32 | &types::F64 => true,
            ty if ty.is_vector() => true,
            _ => false,
        }
    }).collect();
    inl_ty_intersect("ty_float_or_vec", &intersect_types, lin_result, cur_idx)
}

// ty_vector_float: specialize typevar
fn inl_ty_vector_float(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.is_vector() && ty.lane_type().is_float() {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_vector_float", &intersect_types, lin_result, cur_idx)
}

// ty_vector_not_float: specialize typevar
fn inl_ty_vector_not_float(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.is_vector() && !ty.lane_type().is_float() {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_vector_not_float", &intersect_types, lin_result, cur_idx)
}

// ty_vec64: specialize typevar
fn inl_ty_vec64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.is_vector() && ty.bits() == 64 {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_vec64", &intersect_types, lin_result, cur_idx)
        .or_else(|| inl_ty_intersect("ty_vec64_ctor", &intersect_types, lin_result, cur_idx))
}

// ty_vec128: specialize typevar
fn inl_ty_vec128(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.is_vector() && ty.bits() == 128 {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_vec128", &intersect_types, lin_result, cur_idx)
}

// ty_dyn_vec64: specialize typevar
fn inl_ty_dyn_vec64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        match ty {
            &types::I8X8XN | &types::I8X16XN | &types::I16X4XN | &types::I16X8XN | &types::I32X2XN | 
            &types::I32X4XN | &types::I64X2XN | &types::F32X4XN | &types::F64X2XN => (),
            _ => { return false; },
        }
        if dynamic_to_fixed(ty.clone()).bits() == 64 {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_dyn_vec64", &intersect_types, lin_result, cur_idx)
}

// ty_dyn_vec128: specialize typevar
fn inl_ty_dyn_vec128(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        match ty {
            &types::I8X8XN | &types::I8X16XN | &types::I16X4XN | &types::I16X8XN | &types::I32X2XN | 
            &types::I32X4XN | &types::I64X2XN | &types::F32X4XN | &types::F64X2XN => (),
            _ => { return false; },
        }
        if dynamic_to_fixed(ty.clone()).bits() == 128 {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_dyn_vec128", &intersect_types, lin_result, cur_idx)
}

// ty_vec64_int: specialize typevar
fn inl_ty_vec64_int(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.is_vector() && ty.bits() == 64 && ty.lane_type().is_int() {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_vec64_int", &intersect_types, lin_result, cur_idx)
}

// ty_vec128_int: specialize typevar
fn inl_ty_vec128_int(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.is_vector() && ty.bits() == 128 && ty.lane_type().is_int() {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_vec128_int", &intersect_types, lin_result, cur_idx)
}

// ty_addr64: specialize typevar
fn inl_ty_addr64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = vec![types::I64, types::R64];
    inl_ty_intersect("ty_addr64", &intersect_types, lin_result, cur_idx)
}

// ty_dyn64_int: specialize typevar
fn inl_ty_dyn64_int(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.is_dynamic_vector() && ty.min_bits() == 64 && ty.lane_type().is_int() {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_dyn64_int", &intersect_types, lin_result, cur_idx)
}

// ty_dyn128_int: specialize typevar
fn inl_ty_dyn128_int(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.is_dynamic_vector() && ty.min_bits() == 128 && ty.lane_type().is_int() {
            true
        } else {
            false
        }
    }).collect();
    inl_ty_intersect("ty_dyn128_int", &intersect_types, lin_result, cur_idx)
}

// not_vec32x2: specialize typevar
fn inl_not_vec32x2(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let intersect_types = get_all_types().into_iter().filter(|ty| {
        if ty.lane_bits() == 32 && ty.lane_count() == 2 {
            false
        } else {
            true
        }
    }).collect();
    inl_ty_intersect("not_vec32x2", &intersect_types, lin_result, cur_idx)
}

// not_i64x2: specialize typevar, but with the automatically added one
fn inl_not_i64x2(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    if !lin_result.rule.is_lower {
        return None;
    }
    
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "not_i64x2" {
                assert!(params.len() == 1);
                let intersect_types = get_all_types().into_iter().filter(|ty| {
                    if ty == &types::I64X2 {
                        false
                    } else {
                        true
                    }
                }).collect();
                match check_typevar_and_intersect(lin_result.deep_clone(), &params[0], intersect_types) {
                    Some(mut x) => {
                        let new_params = params.into_iter().map(|idx_ref| x.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                        assert!(new_params.len() == 1);
                        *x.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                        Some(vec![x])
                    },
                    None => None
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// lane_type: specialize typevar
fn inl_lane_type(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let mut intersect_types = HashSet::new();
    for ty in get_all_types() {
        intersect_types.insert(ty.lane_type());
    }
    inl_ty_intersect("lane_type", &intersect_types.into_iter().collect(), lin_result, cur_idx)
}

// ty_half_lanes: specialize typevar
fn inl_ty_half_lanes(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let mut intersect_types = HashSet::new();
    for ty in get_all_types().into_iter().filter(|ty| ty.lane_count() != 1) {
        match ty.lane_type().by(ty.lane_count() / 2) {
            Some(ty_res) => { intersect_types.insert(ty_res); },
            None => (),
        }
    }
    inl_ty_intersect("ty_half_lanes", &intersect_types.into_iter().collect(), lin_result, cur_idx)
}

// ty_half_width: specialize typevar
fn inl_ty_half_width(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let mut intersect_types = HashSet::new();
    for ty in get_all_types() {
        let ty_res = match ty.lane_type() {
            types::I16 => types::I8.by(ty.lane_count()),
            types::I32 => types::I16.by(ty.lane_count()),
            types::I64 => types::I32.by(ty.lane_count()),
            types::F64 => types::F32.by(ty.lane_count()),
            _ => None,
        };
        match ty_res {
            Some(x) => { intersect_types.insert(x); },
            None => (),
        }
    }
    inl_ty_intersect("ty_half_lanes", &intersect_types.into_iter().collect(), lin_result, cur_idx)
}

// u64_from_imm64: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm64
fn inl_u64_from_imm64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm64_types().to_vec();
    inl_imm("u64_from_imm64", &types, lin_result, cur_idx)
}

// u128_from_immediate: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm128
fn inl_u128_from_immediate(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm128_types().to_vec();
    inl_imm("u128_from_immediate", &types, lin_result, cur_idx)
}

// imm64: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm64
fn inl_imm64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm64_types().to_vec();
    inl_imm("imm64", &types, lin_result, cur_idx)
}

// imm64_masked: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm64 (ignore masking for simplicity)
fn inl_imm64_masked(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "imm64_masked" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                new_lin_result = check_typevar_and_intersect(new_lin_result, &params[is_lower_int], get_imm64_types().to_vec())?;
                let new_params = params.into_iter().skip(1 + is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// simm32: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm32
fn inl_simm32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm32_types().to_vec();
    inl_imm("simm32", &types, lin_result, cur_idx)
}

// simm32_from_value: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm32
fn inl_simm32_from_value(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm32_types().to_vec();
    inl_imm("simm32_from_value", &types, lin_result, cur_idx)
}

// simm32_from_imm64: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm32
fn inl_simm32_from_imm64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm32_types().to_vec();
    inl_imm("simm32_from_imm64", &types, lin_result, cur_idx)
}

// uimm8: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm8
fn inl_uimm8(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = vec![types::I8];
    inl_imm("uimm8", &types, lin_result, cur_idx)
}

// offset32: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm32
fn inl_offset32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm32_types().to_vec();
    inl_imm("offset32", &types, lin_result, cur_idx)
}

// offset32_to_u32: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm32
fn inl_offset32_to_u32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm32_types().to_vec();
    inl_imm("offset32_to_u32", &types, lin_result, cur_idx)
}

// u32_to_offset32: typecast integer (nop in the expr), specialize typevar of predec. iconst to imm32
fn inl_u32_to_offset32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm32_types().to_vec();
    inl_imm("u32_to_offset32", &types, lin_result, cur_idx)
}

// u32_from_ieee32: float from bit repr. (nop in the expr), specialize typevar of predec. const to F32
fn inl_u32_from_ieee32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = vec![types::F32];
    inl_imm("u32_from_ieee32", &types, lin_result, cur_idx)
}

// u64_from_ieee64: float from bit repr. (nop in the expr), specialize typevar of predec. const to F64
fn inl_u64_from_ieee64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = vec![types::F64];
    inl_imm("u64_from_ieee64", &types, lin_result, cur_idx)
}

// emit_u64_le_const: nop
fn inl_emit_u64_le_const(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("emit_u64_le_const", lin_result, cur_idx)
}

// emit_u128_le_const: nop
fn inl_emit_u128_le_const(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("emit_u128_le_const", lin_result, cur_idx)
}

// dynamic_lane: mix two consts together as a dynamic vector
fn inl_dynamic_lane(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "dynamic_lane" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let lane_bits_stmt = lin_result.get(&params[is_lower_int])?;
                let lane_count_stmt = lin_result.get(&params[is_lower_int + 1])?;
                let mut new_lin_result = lin_result.deep_clone();
                match (lane_bits_stmt, lane_count_stmt) {
                    (LinExpr::Const(val0), LinExpr::Const(val1)) => {
                        let ty_vec: Vec<_> = get_all_types().into_iter().filter(|x|
                            x.is_dynamic_vector() &&
                            x.lane_bits() == (*val0).try_into().unwrap_or(u32::MAX) &&
                            x.min_lane_count() == (*val1).try_into().unwrap_or(u32::MAX)
                        ).collect();
                        assert!(ty_vec.len() > 0);
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(ty_vec);
                        Some(vec![new_lin_result])
                    },
                    (LinExpr::Const(val0), _) => {
                        let ty_vec: Vec<_> = get_all_types().into_iter().filter(|x|
                            x.is_dynamic_vector() &&
                            x.lane_bits() == (*val0).try_into().unwrap_or(u32::MAX)
                        ).collect();
                        assert!(ty_vec.len() > 0);
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(ty_vec);
                        Some(vec![new_lin_result])
                    },
                    (_, LinExpr::Const(val1)) => {
                        let ty_vec: Vec<_> = get_all_types().into_iter().filter(|x|
                            x.is_dynamic_vector() &&
                            x.min_lane_count() == (*val1).try_into().unwrap_or(u32::MAX)
                        ).collect();
                        assert!(ty_vec.len() > 0);
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(ty_vec);
                        Some(vec![new_lin_result])
                    },
                    _ => {
                        let ty_vec: Vec<_> = get_all_types().into_iter().filter(|x|
                            x.is_dynamic_vector()
                        ).collect();
                        assert!(ty_vec.len() > 0);
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(ty_vec);
                        Some(vec![new_lin_result])
                    },
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// multi_lane: mix two consts together as a SIMD var
fn inl_multi_lane(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "multi_lane" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let lane_bits_stmt = lin_result.get(&params[is_lower_int])?;
                let lane_count_stmt = lin_result.get(&params[is_lower_int + 1])?;
                let mut new_lin_result = lin_result.deep_clone();
                match (lane_bits_stmt, lane_count_stmt) {
                    (LinExpr::Const(val0), LinExpr::Const(val1)) => {
                        let ty_vec: Vec<_> = get_all_types().into_iter().filter(|x|
                            x.lane_bits() == (*val0).try_into().unwrap_or(u32::MAX) &&
                            x.lane_count() == (*val1).try_into().unwrap_or(u32::MAX)
                        ).collect();
                        assert!(ty_vec.len() > 0);
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(ty_vec);
                        Some(vec![new_lin_result])
                    },
                    (LinExpr::Const(val0), _) => {
                        let ty_vec: Vec<_> = get_all_types().into_iter().filter(|x|
                            x.lane_bits() == (*val0).try_into().unwrap_or(u32::MAX) &&
                            x.lane_count() > 1
                        ).collect();
                        assert!(ty_vec.len() > 0);
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(ty_vec);
                        Some(vec![new_lin_result])
                    },
                    (_, LinExpr::Const(val1)) => {
                        let ty_vec: Vec<_> = get_all_types().into_iter().filter(|x|
                            x.lane_count() == (*val1).try_into().unwrap_or(u32::MAX)
                        ).collect();
                        assert!(ty_vec.len() > 0);
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(ty_vec);
                        Some(vec![new_lin_result])
                    },
                    _ => {
                        let ty_vec: Vec<_> = get_all_types().into_iter().filter(|x|
                            x.lane_count() > 1
                        ).collect();
                        assert!(ty_vec.len() > 0);
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::TypeVar(ty_vec);
                        Some(vec![new_lin_result])
                    },
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// i64_sextend_imm64: specialize typevar to imm64
fn inl_i64_sextend_imm64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "i64_sextend_imm64" || name == "i64_sextend_u64" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let mut new_lin_result = match check_typevar_and_intersect(
                    lin_result.deep_clone(),
                    &params[is_lower_int],
                    get_imm64_types().to_vec()
                ) {
                    Some(x) => x,
                    None => lin_result.deep_clone(),
                };
                let new_params = params.into_iter().skip(1 + is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// u64_uextend_imm64: specialize typevar to imm64
fn inl_u64_uextend_imm64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "u64_uextend_imm64" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let mut new_lin_result = match check_typevar_and_intersect(
                    lin_result.deep_clone(),
                    &params[is_lower_int],
                    get_imm64_types().to_vec()
                ) {
                    Some(x) => x,
                    None => lin_result.deep_clone(),
                };
                let new_params = params.into_iter().skip(1 + is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// subsume: nop
fn inl_subsume(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("subsume", lin_result, cur_idx)
}

// remat: nop
fn inl_remat(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("remat", lin_result, cur_idx)
}

// splat64: splat64 v => v | v << 64
fn inl_splat64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("splat64", "_splat64", lin_result, cur_idx)
}

// ty_as_ty: nop
fn inl_ty_as_ty(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("u8_as_u32", lin_result, cur_idx)
        .or_else(|| inl_identity("u8_as_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("u16_as_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("u32_as_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("i64_as_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("u64_as_i32", lin_result, cur_idx))
        .or_else(|| inl_identity("u128_as_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("u64_as_u32", lin_result, cur_idx))
        .or_else(|| inl_identity("u64_as_i16", lin_result, cur_idx))
}

// ty_from_ty with nop
fn inl_ty_from_ty_identity(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("imm12_from_u64", lin_result, cur_idx)
        .or_else(|| inl_identity("imm12_from_i64", lin_result, cur_idx))
        .or_else(|| inl_identity("imm5_from_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("imm5_from_i8", lin_result, cur_idx))
        .or_else(|| inl_identity("uimm5_bitcast_to_imm5", lin_result, cur_idx))
        .or_else(|| inl_identity("uimm5_from_u8", lin_result, cur_idx))
        .or_else(|| inl_identity("uimm5_from_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("imm_shift_from_u8", lin_result, cur_idx))
        .or_else(|| inl_identity("imm_from_bits", lin_result, cur_idx))
        .or_else(|| inl_identity("imm_from_neg_bits", lin_result, cur_idx))
        .or_else(|| inl_identity_2("imm_shift_from_imm64", lin_result, cur_idx))
        .or_else(|| inl_identity_2("u64_into_imm_logic", lin_result, cur_idx))
        .or_else(|| inl_identity("u8_from_uimm8", lin_result, cur_idx))
        .or_else(|| inl_identity("vconst_from_immediate", lin_result, cur_idx))
        .or_else(|| inl_identity("vec_mask_from_immediate", lin_result, cur_idx))
        .or_else(|| inl_identity("u64_from_constant", lin_result, cur_idx))
        .or_else(|| inl_identity("u128_from_constant", lin_result, cur_idx))
        .or_else(|| inl_identity("const_to_vconst", lin_result, cur_idx))
        .or_else(|| inl_identity("u8_into_uimm5", lin_result, cur_idx))
        .or_else(|| inl_identity("u8_into_imm12", lin_result, cur_idx))
        .or_else(|| inl_identity("i32_from_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("i16_from_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("i16_from_u32", lin_result, cur_idx))
        .or_else(|| inl_identity("uimm32shifted_from_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("uimm16shifted_from_u64", lin_result, cur_idx))
        .or_else(|| inl_identity("shuffle_mask_from_u128", lin_result, cur_idx))
        .or_else(|| inl_identity("u64_from_value", lin_result, cur_idx))
        .or_else(|| inl_identity("u32_from_value", lin_result, cur_idx))
        .or_else(|| inl_identity("u8_from_value", lin_result, cur_idx))
        .or_else(|| inl_identity("u8_from_signed_value", lin_result, cur_idx))
        .or_else(|| inl_identity("i64_from_value", lin_result, cur_idx))
        .or_else(|| inl_identity("i32_from_value", lin_result, cur_idx))
        .or_else(|| inl_identity("i16_from_value", lin_result, cur_idx))
        .or_else(|| inl_identity("uimm16shifted_from_value", lin_result, cur_idx))
        .or_else(|| inl_identity("uimm32shifted_from_value", lin_result, cur_idx))
        .or_else(|| inl_identity("i64_from_offset", lin_result, cur_idx))
}

// shift_amount_masked: nop
fn inl_shift_amount_masked(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity_2("shift_amount_masked", lin_result, cur_idx)
}

// imm12_const: nop
fn inl_imm12_const(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("imm12_const", lin_result, cur_idx)
}

// offset32_imm: nop
fn inl_offset32_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("offset32_imm", lin_result, cur_idx)
}

// u64_truncate_to_u32: nop
fn inl_u64_truncate_to_u32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity("u64_truncate_to_u32", lin_result, cur_idx)
}

// shift_masked_imm: nop
fn inl_shift_masked_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_identity_2("shift_masked_imm", lin_result, cur_idx)
}

// value_type: (value_type ty) -> (identity ty new_var): identity defines type of new_var as ty
fn inl_value_type(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "value_type" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = lin_result.deep_clone();
                if lin_result.rule.is_lower {
                    match lin_result.get(&params[is_lower_int])? {
                        LinExpr::TypeVar(types) => {
                            let predecs = new_lin_result.get_predecessors(&cur_idx);
                            for predec in predecs {
                                match new_lin_result.get(&predec)? {
                                    LinExpr::Expr { name: _, params } => {
                                        match check_typevar_and_intersect(new_lin_result.deep_clone(), &params[0], types.clone()) {
                                            Some(x) => { new_lin_result = x; },
                                            None => (),
                                        }
                                    },
                                    _ => ()
                                }
                            }
                            *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new());
                        },
                        _ => {
                            *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new());
                        }
                    }
                }
                else {
                    *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new()); // ignore types: they are not applicable
                }
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// imm64_power_of_two: add constraint to the param, specialize typevar of predec. iconst to imm64
fn inl_imm64_power_of_two(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "imm64_power_of_two" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                new_lin_result = check_predec_and_intersect_const_types(
                    new_lin_result, 
                    cur_idx,
                    get_imm64_types().to_vec()
                )?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_pow2"), params: new_params });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new());
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// u64_nonzero: add constraint to the param, specialize typevar of predec. iconst to imm64
fn inl_u64_nonzero(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm64_types().to_vec();
    inl_novarcond_imm("u64_nonzero", "_nonzero", &types, lin_result, cur_idx)
        .or_else(|| inl_novarcond_imm("i64_nonzero", "_nonzero", &types, lin_result, cur_idx))
        .or_else(|| inl_novarcond_imm("nonzero_u64_from_imm64", "_nonzero", &types, lin_result, cur_idx))
        .or_else(|| inl_novarcond_imm("u64_nonzero_hipart", "_nonzero_hipart", &types, lin_result, cur_idx))
        .or_else(|| inl_novarcond_imm("u64_nonzero_lopart", "_nonzero_lopart", &types, lin_result, cur_idx))
}

// nonzero_u64_fits_in_u32: add constraint to the param, specialize typevar of predec. iconst to imm64
fn inl_nonzero_u64_fits_in_u32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "nonzero_u64_fits_in_u32" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                new_lin_result = check_predec_and_intersect_const_types(
                    new_lin_result, 
                    cur_idx, 
                    get_imm64_types().to_vec()
                )?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_nonzero"), params: Vec::new() });
                let cond_idx2 = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_fits_in_32"), params: Vec::new() });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx2)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// vconst_all_ones_or_all_zeros: add constraint to the param, specialize typevar of predec. iconst to imm64
fn inl_vconst_all_ones_or_all_zeros(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "vconst_all_ones_or_all_zeros" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == is_lower_int);
                let mut new_lin_result = lin_result.deep_clone();
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_vconst_all_ones_or_all_zeros"), params: Vec::new() });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new());
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// pshufd_lhs_imm: add constraint to the param
fn inl_pshufd_lhs_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("pshufd_lhs_imm", "_pshufd_lhs_imm", lin_result, cur_idx)
}

// pshufd_rhs_imm: add constraint to the param
fn inl_pshufd_rhs_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("pshufd_rhs_imm", "_pshufd_rhs_imm", lin_result, cur_idx)
}

// shufps_imm: add constraint to the param
fn inl_shufps_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("shufps_imm", "_shufps_imm", lin_result, cur_idx)
}

// shufps_rev_imm: add constraint to the param
fn inl_shufps_rev_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("shufps_rev_imm", "_shufps_rev_imm", lin_result, cur_idx)
}

// pshuflw_lhs_imm: add constraint to the param
fn inl_pshuflw_lhs_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("pshuflw_lhs_imm", "_pshuflw_lhs_imm", lin_result, cur_idx)
}

// pshuflw_rhs_imm: add constraint to the param
fn inl_pshuflw_rhs_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("pshuflw_rhs_imm", "_pshuflw_rhs_imm", lin_result, cur_idx)
}

// pshufhw_lhs_imm: add constraint to the param
fn inl_pshufhw_lhs_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("pshufhw_lhs_imm", "_pshufhw_lhs_imm", lin_result, cur_idx)
}

// pshufhw_rhs_imm: add constraint to the param
fn inl_pshufhw_rhs_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("pshufhw_rhs_imm", "_pshufhw_rhs_imm", lin_result, cur_idx)
}

// palignr_imm_from_immediate: add constraint to the param
fn inl_palignr_imm_from_immediate(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("palignr_imm_from_immediate", "_palignr_imm_from_immediate", lin_result, cur_idx)
}

// pblendw_imm: add constraint to the param
fn inl_pblendw_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_novarcond("pblendw_imm", "_pblendw_imm", lin_result, cur_idx)
}

// u32_nonnegative: add constraint to the param, specialize typevar of predec. iconst to imm32
fn inl_u32_nonnegative(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let types = get_imm32_types().to_vec();
    inl_novarcond_imm("u32_nonnegative", "_nonnegative", &types, lin_result, cur_idx)
}

// sse_interps_lane_imm: add constraint to the param, 0 | lane << 4
fn inl_sse_interps_lane_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("sse_interps_lane_imm", "_sse_interps_lane_imm", lin_result, cur_idx)
}

// u64_from_inverted_value: add constraint to the param
fn inl_u64_from_inverted_value(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("u64_from_inverted_value", "_inverted", lin_result, cur_idx)
}

// uimm16shifted_from_inverted_value: add constraint to the param
fn inl_uimm16shifted_from_inverted_value(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("uimm16shifted_from_inverted_value", "_inverted", lin_result, cur_idx)
}

// uimm32shifted_from_inverted_value: add constraint to the param
fn inl_uimm32shifted_from_inverted_value(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("uimm32shifted_from_inverted_value", "_inverted", lin_result, cur_idx)
}

// i16_from_swapped_value: add constraint to the param
fn inl_i16_from_swapped_value(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("i16_from_swapped_value", "_swapped", lin_result, cur_idx)
}

// i64_from_negated_value: add constraint to the param
fn inl_i64_from_negated_value(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("i64_from_negated_value", "_negate", lin_result, cur_idx)
}

// i32_from_negated_value: add constraint to the param
fn inl_i32_from_negated_value(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("i32_from_negated_value", "_negate", lin_result, cur_idx)
}

// i16_from_negated_value: add constraint to the param
fn inl_i16_from_negated_value(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("i16_from_negated_value", "_negate", lin_result, cur_idx)
}

// len_minus_one: add constraint to the param
fn inl_len_minus_one(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("len_minus_one", "_minusone", lin_result, cur_idx)
}

// shuffle64_from_imm: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_shuffle64_from_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "shuffle64_from_imm" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 2);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_shuffle64_from_imm"), params: new_params });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new());
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// shuffle32_from_imm: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_shuffle32_from_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "shuffle32_from_imm" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 4 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 4);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_shuffle32_from_imm"), params: new_params });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new());
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// shuffle16_from_imm: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_shuffle16_from_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "shuffle16_from_imm" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 8 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 8);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_shuffle16_from_imm"), params: new_params });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new());
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// shuffle_dup8_from_imm: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_shuffle_dup8_from_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("shuffle_dup8_from_imm", "_shuffle_dup8_from_imm", lin_result, cur_idx)
}

// shuffle_dup16_from_imm: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_shuffle_dup16_from_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("shuffle_dup16_from_imm", "_shuffle_dup16_from_imm", lin_result, cur_idx)
}

// shuffle_dup32_from_imm: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_shuffle_dup32_from_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("shuffle_dup32_from_imm", "_shuffle_dup32_from_imm", lin_result, cur_idx)
}

// shuffle_dup64_from_imm: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_shuffle_dup64_from_imm(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("shuffle_dup64_from_imm", "_shuffle_dup64_from_imm", lin_result, cur_idx)
}

// u64_low32_bits_unset: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_u64_low32_bits_unset(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("u64_low32_bits_unset", "_u64_low32_bits_unset", lin_result, cur_idx)
}

// u128_replicated_u64: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_u128_replicated_u64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("u128_replicated_u64", "_u128_replicated_u64", lin_result, cur_idx)
}

// u64_replicated_u32: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_u64_replicated_u32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("u64_replicated_u32", "_u64_replicated_u32", lin_result, cur_idx)
}

// u32_replicated_u16: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_u32_replicated_u16(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("u32_replicated_u16", "_u32_replicated_u16", lin_result, cur_idx)
}

// u16_replicated_u8: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_u16_replicated_u8(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("u16_replicated_u8", "_u16_replicated_u8", lin_result, cur_idx)
}

// neg_imm12: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_neg_imm12(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    inl_varcond_1("inl_imm5_from_i8", "_neg", lin_result, cur_idx)
}

// safe_divisor_from_imm64: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_safe_divisor_from_imm64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "safe_divisor_from_imm64" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let mut new_lin_result = lin_result.deep_clone();
                let new_params = params.into_iter().skip(1 + is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_nonzero"), params: Vec::new() });
                let cond_idx2 = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from("_nonminusone"), params: Vec::new() });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx2)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// has_type: apply type to latter expression
fn inl_has_type(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "has_type" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let mut new_lin_result = lin_result.deep_clone();
                
                // change var to typevar (if possible)
                match new_lin_result.get(&params[is_lower_int]).unwrap() {
                    LinExpr::Var(_) => {
                        *new_lin_result.get_mut(&params[is_lower_int]).unwrap() = LinExpr::TypeVar(get_all_types().to_vec());
                    },
                    _ => (),
                }

                // adjust parameters
                let typevar_idx = new_lin_result.get_idx(&params[is_lower_int])?;
                match new_lin_result.get_mut(&params[1 + is_lower_int]).unwrap() {
                    LinExpr::Expr { name: _, params } => {
                        params[0] = typevar_idx;
                    },
                    _ => { return None; },
                }

                // change has_type to VP
                let expr_idx = new_lin_result.get_idx(&params[1 + is_lower_int])?;
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: vec![expr_idx] };
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// ty_bits: group typevars with ty.bits(), change expression to const of ty.bits()
fn inl_ty_bits(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| Some(ty.bits() as i128);
    inl_ty_sep("ty_bits", func, lin_result, cur_idx)
        .or_else(|| inl_ty_sep("ty_bits_u16", func, lin_result, cur_idx))
        .or_else(|| inl_ty_sep("ty_bits_u64", func, lin_result, cur_idx))
}

// ty_bytes: group typevars with ty.bytes(), change expression to const of ty.bytes()
fn inl_ty_bytes(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| Some(ty.bytes() as i128);
    inl_ty_sep("ty_bytes", func, lin_result, cur_idx)
}

// ty_mask: group typevars with mask, change expression to const of mask
fn inl_ty_mask(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| {
        if ty.bits() > 64 || ty.bits() == 0 { None }
        else {
            let ty_shift = 64_u64.checked_sub(ty.bits().into()).unwrap();
            Some((u64::MAX >> ty_shift) as i128)
        }
    };
    inl_ty_sep("ty_mask", func, lin_result, cur_idx)
}

// ty_lane_mask: group typevars with lane mask, change expression to const of lane mask
fn inl_ty_lane_mask(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| {
        if ty.lane_count() > 64 || ty.lane_count() == 0 { None }
        else {
            let ty_shift = 64_u64.checked_sub(ty.lane_count().into()).unwrap();
            Some((u64::MAX >> ty_shift) as i128)
        }
    };
    inl_ty_sep("ty_lane_mask", func, lin_result, cur_idx)
}

// ty_lane_count: group typevars with lane count, change expression to const of lane count
fn inl_ty_lane_count(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| Some(ty.lane_count() as i128);
    inl_ty_sep("ty_lane_count", func, lin_result, cur_idx)
}

// ty_umin: 0
fn inl_ty_umin(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "ty_umin" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = lin_result.deep_clone();
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(0);
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// ty_umax: ty_mask
fn inl_ty_umax(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| {
        if ty.bits() > 64 || ty.bits() == 0 { None }
        else {
            let ty_shift = 64_u64.checked_sub(ty.bits().into()).unwrap();
            Some((u64::MAX >> ty_shift) as i128)
        }
    };
    inl_ty_sep("ty_umax", func, lin_result, cur_idx)
}

// ty_smin: group typevars with smin, change expression to const of smin
fn inl_ty_smin(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| {
        if ty.bits() > 64 || ty.bits() == 0 { None }
        else {
            let ty_shift = 64_u64.checked_sub(ty.bits().into()).unwrap();
            Some(((i64::MIN as u64) >> ty_shift) as i128)
        }
    };
    inl_ty_sep("ty_smin", func, lin_result, cur_idx)
}

// ty_smax: group typevars with smax, change expression to const of smax
fn inl_ty_smax(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| {
        if ty.bits() > 64 || ty.bits() == 0 { None }
        else {
            let ty_shift = 64_u64.checked_sub(ty.bits().into()).unwrap();
            Some(((i64::MAX as u64) >> ty_shift) as i128)
        }
    };
    inl_ty_sep("ty_smax", func, lin_result, cur_idx)
}

// shift_mask: group typevars with shift mask, change expression to const of shift mask
fn inl_shift_mask(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let func = |ty: &Type| Some((ty.lane_bits().checked_sub(1)?) as i128);
    inl_ty_sep("shift_mask", func, lin_result, cur_idx)
}

// zero_offset: const(0)
fn inl_zero_offset(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "zero_offset" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 0 + is_lower_int);
                let mut new_lin_result = lin_result.deep_clone();
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(0);
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// fcvt_to_uint_ub32
fn inl_fcvt_to_uint_ub32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "fcvt_to_uint_ub32" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                match new_lin_result.get(&params[is_lower_int]).unwrap() {
                    LinExpr::Const(val) => {
                        let new_val = (2.0_f32).powi((*val as u8).into()).to_bits() as i128;
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(new_val);
                        Some(vec![new_lin_result])
                    },
                    _ => None
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// fcvt_to_uint_lb32
fn inl_fcvt_to_uint_lb32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "fcvt_to_uint_lb32" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 0 + is_lower_int);
                let mut new_lin_result = lin_result.deep_clone();
                let new_val = (-1.0_f32).to_bits() as i128;
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(new_val);
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// fcvt_to_uint_ub64
fn inl_fcvt_to_uint_ub64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "fcvt_to_uint_ub64" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                match new_lin_result.get(&params[is_lower_int]).unwrap() {
                    LinExpr::Const(val) => {
                        let new_val = (2.0_f64).powi((*val as u8).into()).to_bits() as i128;
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(new_val);
                        Some(vec![new_lin_result])
                    },
                    _ => None
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// fcvt_to_uint_lb64
fn inl_fcvt_to_uint_lb64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "fcvt_to_uint_lb64" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 0 + is_lower_int);
                let mut new_lin_result = lin_result.deep_clone();
                let new_val = (-1.0_f64).to_bits() as i128;
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(new_val);
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// fcvt_to_sint_ub32
fn inl_fcvt_to_sint_ub32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "fcvt_to_sint_ub32" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                match new_lin_result.get(&params[is_lower_int]).unwrap() {
                    LinExpr::Const(val) => {
                        let new_val = (2.0_f32).powi(((*val as u8).checked_sub(1)?).into()).to_bits() as i128;
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(new_val);
                        Some(vec![new_lin_result])
                    },
                    _ => None
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// fcvt_to_sint_lb32
fn inl_fcvt_to_sint_lb32(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "fcvt_to_sint_lb32" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                match new_lin_result.get(&params[is_lower_int]).unwrap() {
                    LinExpr::Const(val) => {
                        let lb = (-2.0_f32).powi(((*val as u8).checked_sub(1)?).into());
                        let new_val = std::cmp::max(lb.to_bits() + 1, (lb - 1.0).to_bits()) as i128;
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(new_val);
                        Some(vec![new_lin_result])
                    },
                    _ => None
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// fcvt_to_sint_ub64
fn inl_fcvt_to_sint_ub64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "fcvt_to_sint_ub64" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                match new_lin_result.get(&params[is_lower_int]).unwrap() {
                    LinExpr::Const(val) => {
                        let new_val = (2.0_f64).powi(((*val as u8).checked_sub(1)?).into()).to_bits() as i128;
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(new_val);
                        Some(vec![new_lin_result])
                    },
                    _ => None
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// fcvt_to_sint_lb64
fn inl_fcvt_to_sint_lb64(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "fcvt_to_sint_lb64" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                match new_lin_result.get(&params[is_lower_int]).unwrap() {
                    LinExpr::Const(val) => {
                        let lb = (-2.0_f64).powi(((*val as u8).checked_sub(1)?).into());
                        let new_val = std::cmp::max(lb.to_bits() + 1, (lb - 1.0).to_bits()) as i128;
                        *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(new_val);
                        Some(vec![new_lin_result])
                    },
                    _ => None
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// i64_from_iconst: construct iconst
fn inl_u64_from_iconst(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == "i64_from_iconst" || name == "u64_from_iconst" {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                let mut new_params = params.into_iter().skip(is_lower_int).map(
                    |idx_ref| new_lin_result.get_idx(idx_ref)
                ).collect::<Option<Vec<_>>>()?;
                let expr_idx = new_lin_result.get_idx(cur_idx).unwrap();
                let typevar_idx = cur_idx.deep_clone();
                new_lin_result.insert_pair(typevar_idx.clone(), LinExpr::TypeVar(get_imm64_types().to_vec())); // typevar
                new_params.insert(0, typevar_idx);
                *new_lin_result.get_mut(&expr_idx)? = LinExpr::Expr { name: String::from("iconst"), params: new_params };
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// reverts typevar back to var
fn inl_revert_typevar(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if TYPEVAR_REVERT_LIST.contains(&name.as_str()) ||
               name.starts_with("IntCC.") || name.starts_with("FloatCC.") {
                let typevar_idx = params.get(0)?;
                let typevar_stmt = lin_result.get(typevar_idx)?;
                match typevar_stmt {
                    LinExpr::TypeVar(_) => {
                        let mut new_lin_result = lin_result.deep_clone();
                        if lin_result.rule.is_lower {
                            let new_params = params.into_iter().skip(1).map(
                                |idx_ref| new_lin_result.get_idx(idx_ref)
                            ).collect::<Option<Vec<_>>>()?;
                            *new_lin_result.get_mut(cur_idx)? = LinExpr::Expr { name: name.clone(), params: new_params }
                        }
                        else {
                            *new_lin_result.get_mut(typevar_idx)? = LinExpr::Var(Vec::new()); // typevar
                        }
                        Some(vec![new_lin_result])
                    },
                    _ => None,
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// utilities
fn check_typevar_and_intersect(lin_result: LinResult, typevar_idx: &LinExprIdx, intersect_types: Vec<Type>) -> Option<LinResult> {
    let typevar_stmt = lin_result.get(typevar_idx)?;
    match typevar_stmt {
        LinExpr::TypeVar(types) => {
            let new_types = get_types_intersection(types.clone(), intersect_types);
            if new_types.len() == 0 {
                return None;
            }
            let mut new_lin_result = lin_result;
            *new_lin_result.get_mut(typevar_idx)? = LinExpr::TypeVar(new_types); // typevar
            Some(new_lin_result)
        },
        LinExpr::Var(_) => { // in lowering, types are first expressed as Var
            let mut new_lin_result = lin_result;
            *new_lin_result.get_mut(typevar_idx)? = LinExpr::TypeVar(intersect_types); // typevar
            Some(new_lin_result)
        }
        _ => Some(lin_result),
    }
}

fn check_predec_and_intersect_const_types(lin_result: LinResult, cur_idx: &LinExprIdx, intersect_types: Vec<Type>) -> Option<LinResult> {
    let predec_vec = lin_result.get_predecessors(cur_idx);
    let mut new_lin_result = lin_result;
    
    for predec in predec_vec {
        new_lin_result = match new_lin_result.get(&predec)? {
            LinExpr::Expr { name, params } 
                if name == "iconst" || name == "f32const" || name == "f64const" => {
                check_typevar_and_intersect(new_lin_result.clone(), &params[0], intersect_types.clone())?
            },
            _ => new_lin_result,
        }
    }

    Some(new_lin_result)
}

// make typevar to var since the var is wrongly changed to typevar
fn change_typevar_to_var(lin_result: LinResult, typevar_idx: &LinExprIdx) -> Option<LinResult> {
    let typevar_stmt = lin_result.get(typevar_idx)?;
    match typevar_stmt {
        LinExpr::TypeVar(_) => {
            let mut new_lin_result = lin_result;
            *new_lin_result.get_mut(typevar_idx)? = LinExpr::Var(Vec::new()); // typevar
            Some(new_lin_result)
        },
        _ => Some(lin_result),
    }
}

// util for internals that do nothing
fn inl_identity(expr_name: &str, lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == expr_name {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// util for internals that do nothing, but has a type variable
fn inl_identity_2(expr_name: &str, lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == expr_name {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 2 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                let new_params = params.into_iter().skip(1 + is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// util for internals that diverges by ty
fn inl_ty_sep(expr_name: &str, func: impl Fn(&Type) -> Option<i128>, lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == expr_name {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let typevar_stmt = lin_result.get(&params[is_lower_int]).unwrap();
                match typevar_stmt {
                    LinExpr::Var(_) => {
                        // group types by ty.bits
                        let mut results_map: HashMap<i128, Vec<Type>> = HashMap::new();
                        for ty in get_all_types() {
                            let ty_result = match func(&ty) {
                                Some(x) => x,
                                None => { continue; },
                            };
                            results_map.entry(ty_result).or_default().push(ty);
                        }

                        let mut results = Vec::new();
                        for (ty_results, ty_vec) in results_map {
                            let mut new_lin_result = lin_result.deep_clone();
                            *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(ty_results);
                            *new_lin_result.get_mut(&params[is_lower_int])? = LinExpr::TypeVar(ty_vec);
                            results.push(new_lin_result);
                        }
                        Some(results)

                    },
                    LinExpr::TypeVar(types) => {
                        // group types by ty.bits
                        let mut results_map: HashMap<i128, Vec<Type>> = HashMap::new();
                        for ty in types {
                            let ty_result = match func(ty) {
                                Some(x) => x,
                                None => { continue; },
                            };
                            results_map.entry(ty_result).or_default().push(ty.clone());
                        }

                        let mut results = Vec::new();
                        for (ty_results, ty_vec) in results_map {
                            let mut new_lin_result = lin_result.deep_clone();
                            *new_lin_result.get_mut(&cur_idx)? = LinExpr::Const(ty_results);
                            *new_lin_result.get_mut(&params[is_lower_int])? = LinExpr::TypeVar(ty_vec);
                            results.push(new_lin_result);
                        }
                        Some(results)
                    },
                    _ => None,
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

fn inl_ty_intersect(expr_name: &str, intersect_types: &Vec<Type>, lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == expr_name {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                match check_typevar_and_intersect(lin_result.deep_clone(), &params[is_lower_int], intersect_types.clone()) {
                    Some(mut x) => {
                        let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| x.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                        assert!(new_params.len() == 1);
                        *x.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                        Some(vec![x])
                    },
                    None => None
                }
            }
            else {
                None
            }
        },
        _ => None
    }
}

// imm: typecast integer (nop in the expr), specialize typevar of predec. iconst to given types
fn inl_imm(expr_name: &str, intersect_types: &Vec<Type>, lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == expr_name {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                new_lin_result = check_predec_and_intersect_const_types(
                    new_lin_result, 
                    cur_idx, 
                    intersect_types.clone(),
                )?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// novarcond: apply conditions (with no variables) while substituting the expression to a variable
fn inl_novarcond(expr_name: &str, cond_name: &str, lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == expr_name {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from(cond_name), params: Vec::new() });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// novarcond_imm: apply conditions (with no variables) while substituting the expression to a variable
// also checks predec. and intersects with given types
fn inl_novarcond_imm(
    expr_name: &str, 
    cond_name: &str, 
    intersect_types: &Vec<Type>, 
    lin_result: &LinResult, 
    cur_idx: &LinExprIdx
) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == expr_name {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                new_lin_result = check_predec_and_intersect_const_types(
                    new_lin_result, 
                    cur_idx, 
                    intersect_types.clone(),
                )?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from(cond_name), params: Vec::new() });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Expr { name: String::from(VP_NAME), params: new_params };
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

// varcond_1: apply conds (with no vars) while substituting the expr to a var., while the cond is dependent to a param
fn inl_varcond_1(expr_name: &str, cond_name: &str, lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>> {
    let stmt = lin_result.get(&cur_idx)?;
    match stmt {
        LinExpr::Expr { name, params } => {
            if name == expr_name {
                let is_lower_int = lin_result.rule.is_lower as usize;
                assert!(params.len() == 1 + is_lower_int);
                let mut new_lin_result = change_typevar_to_var(lin_result.deep_clone(), &params[is_lower_int])?;
                let new_params = params.into_iter().skip(is_lower_int).map(|idx_ref| new_lin_result.get_idx(idx_ref)).collect::<Option<Vec<_>>>()?;
                assert!(new_params.len() == 1);
                let cond_idx = new_lin_result.cond_stmts.push(LinExpr::Expr { name: String::from(cond_name), params: new_params });
                *new_lin_result.get_mut(&cur_idx)? = LinExpr::Var(Vec::new());
                new_lin_result.cond_pairs.push((new_lin_result.get_idx(cur_idx).unwrap().clone(), LinExprIdx::Cond(cond_idx)));
                Some(vec![new_lin_result])
            }
            else {
                None
            }
        },
        _ => None
    }
}

#[cfg(test)]
mod test {
    use crate::{isle::ISLEParseOptions, isle_lin::linearize_rules_opt};

    use super::*;

    #[test]
    fn test_process_internals() {
        let rules = linearize_rules_opt(ISLEParseOptions::Lower);
        let processed_rules: Vec<_> = rules.into_iter().flat_map(process_internals).collect();
        println!("{:#?}", processed_rules);
    }
}