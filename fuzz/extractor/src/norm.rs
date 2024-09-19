use std::{collections::HashMap, cell::RefCell, rc::Rc};

use cranelift_codegen::ir::{Type, types};
use wasm_ast::Instruction;

use crate::wasm_map::IRData;

// Normalized expressions
pub type NormVar = usize;

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum NormExpr {
    Var(NormVar),
    Expr { name: String, subexprs: Vec<NormExpr> },
    Wildcard,
    ConstInt(i128),
    ConstPrim(String),
    BoundVar(NormVar), // aliases
}

#[derive(Clone, Debug, PartialEq)]
pub struct NormConstraint {
    pub lhs: NormExpr,
    pub rhs: NormExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NormRule {
    pub var_len: usize, // variables are described as indices
    pub is_lower: bool, // is this rule from lowering?
    pub lhs: NormExpr,
    pub rhs: NormExpr,
    pub bound_vars: Vec<NormExpr>,
    pub constraints: Vec<NormConstraint>,
}

// Linearized expressions
#[derive(Clone, Debug, PartialEq, Copy, Hash, Eq)]
pub enum LinType {
    LHS,
    RHS,
    Cond,
}

// Performs deep clone, also for Rc + RefCell
pub trait DeepClone {
    fn deep_clone_impl(
        &self,
        cur_lin_ty: LinType, 
        var_map: &mut HashMap<(LinType, usize), Rc<RefCell<usize>>>
    ) -> Self;
}

impl<T: DeepClone> DeepClone for Vec<T> {
    fn deep_clone_impl(
        &self, 
        cur_lin_ty: LinType, 
        var_map: &mut HashMap<(LinType, usize), Rc<RefCell<usize>>>
    ) -> Self {
        self.into_iter().map(|x| x.deep_clone_impl(cur_lin_ty, var_map)).collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LinExprIdx {
    LHS(Rc<RefCell<usize>>),
    RHS(Rc<RefCell<usize>>),
    Cond(Rc<RefCell<usize>>),
}

impl DeepClone for LinExprIdx {
    fn deep_clone_impl(
        &self, 
        _cur_lin_ty: LinType, 
        var_map: &mut HashMap<(LinType, usize), Rc<RefCell<usize>>>
    ) -> Self {
        let (lin_ty, idx) = match self {
            LinExprIdx::LHS(idx) => (LinType::LHS, *idx.borrow()),
            LinExprIdx::RHS(idx) => (LinType::RHS, *idx.borrow()),
            LinExprIdx::Cond(idx) => (LinType::Cond, *idx.borrow()),
        };
        if var_map.contains_key(&(lin_ty, idx)) {
            let new_idx = var_map.get(&(lin_ty, idx)).unwrap().clone();
            match lin_ty {
                LinType::LHS => LinExprIdx::LHS(new_idx),
                LinType::RHS => LinExprIdx::RHS(new_idx),
                LinType::Cond => LinExprIdx::Cond(new_idx),
            }
        }
        else {
            let new_idx = Rc::new(RefCell::new(idx));
            var_map.insert((lin_ty, idx), new_idx.clone());
            match lin_ty {
                LinType::LHS => LinExprIdx::LHS(new_idx),
                LinType::RHS => LinExprIdx::RHS(new_idx),
                LinType::Cond => LinExprIdx::Cond(new_idx),
            }
        }
    }
}

impl LinExprIdx {
    pub fn deep_clone(&self) -> Self {
        match self {
            LinExprIdx::LHS(idx) => LinExprIdx::LHS(Rc::new(RefCell::new(*idx.borrow()))),
            LinExprIdx::RHS(idx) => LinExprIdx::RHS(Rc::new(RefCell::new(*idx.borrow()))),
            LinExprIdx::Cond(idx) => LinExprIdx::Cond(Rc::new(RefCell::new(*idx.borrow()))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinVec<T> {
    store: Vec<T>,
    idx: Vec<Rc<RefCell<usize>>>,
}

impl<T> LinVec<T> {
    pub fn new() -> Self {
        LinVec {
            store: Vec::new(),
            idx: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        assert!(self.store.len() == self.idx.len());
        self.store.len()
    }

    pub fn push(&mut self, val: T) -> Rc<RefCell<usize>> {
        self.store.push(val);
        let idx = Rc::new(RefCell::new(self.idx.len()));
        self.idx.push(idx.clone());
        assert!(self.store.len() == self.idx.len());
        idx
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.store.get(idx)
    }

    pub fn get_idx_ref(&self, idx: usize) -> Option<&Rc<RefCell<usize>>> {
        self.idx.get(idx)
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.store.get_mut(idx)
    }

    pub fn replace(&mut self, idx: usize, val: T) {
        self.store[idx] = val;
    }

    pub fn remove(&mut self, idx: usize) -> (Rc<RefCell<usize>>, T) {
        assert!(idx < self.store.len());
        let result = self.store.remove(idx);
        let result_idx = self.idx.remove(idx);
        self.refresh_idx(idx);
        assert!(self.store.len() == self.idx.len());
        (result_idx, result)
    }

    pub fn insert(&mut self, idx: usize, val: T) {
        self.store.insert(idx, val);
        self.idx.insert(idx, Rc::new(RefCell::new(usize::MAX))); // dummy index
        self.refresh_idx(idx);
        assert!(self.store.len() == self.idx.len());
    }

    pub fn insert_pair(&mut self, idx: usize, idx_ref: Rc<RefCell<usize>>, val: T) {
        assert!(*idx_ref.borrow() == idx); // idx_ref must be correct
        self.store.insert(idx, val);
        self.idx.insert(idx, idx_ref);
        self.refresh_idx(idx + 1); // skip updating the following idx
        assert!(self.store.len() == self.idx.len());
    }

    // destruct to store and idx
    pub fn destruct(self) -> (Vec<T>, Vec<Rc<RefCell<usize>>>) {
        (self.store, self.idx)
    }

    // construct into LinVec<T>f
    pub fn construct(store: Vec<T>, idx: Vec<Rc<RefCell<usize>>>) -> Self {
        assert!(store.len() == idx.len());
        let mut vec = Self { store, idx };
        vec.refresh_idx(0);
        vec
    }

    fn refresh_idx(&mut self, start_idx: usize) {
        for (idx_iter, idx_iter_ref) in self.idx.iter().enumerate().skip(start_idx) {
            *idx_iter_ref.borrow_mut() = idx_iter;
        }
    }
}

impl<T: DeepClone> DeepClone for LinVec<T> {
    fn deep_clone_impl(
        &self, 
        cur_lin_ty: LinType, 
        var_map: &mut HashMap<(LinType, usize), Rc<RefCell<usize>>>
    ) -> Self {
        let store_cloned = self.store.deep_clone_impl(cur_lin_ty, var_map);
        let idx_cloned = self.idx.iter().map(|x| {
            let cur_idx = *x.borrow();
            if var_map.contains_key(&(cur_lin_ty, cur_idx)) {
                let new_idx = var_map.get(&(cur_lin_ty, cur_idx)).unwrap();
                new_idx.clone()
            }
            else {
                let new_idx = Rc::new(RefCell::new(cur_idx));
                var_map.insert((cur_lin_ty, cur_idx), new_idx.clone());
                new_idx
            }
        }).collect();
        Self {
            store: store_cloned,
            idx: idx_cloned,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LinExpr {
    Var(Vec<Vec<CondExpr>>), // list of rule conditions
    TypeVar(Vec<Type>), // list of possible types
    Const(i128),
    ConstPrim(String),
    Expr { name: String, params: Vec<LinExprIdx> },
    Ident(LinExprIdx), // RHS and Cond may need simple reference (identity) to other expression
}

impl DeepClone for LinExpr {
    fn deep_clone_impl(
        &self, 
        cur_lin_ty: LinType, 
        var_map: &mut HashMap<(LinType, usize), Rc<RefCell<usize>>>
    ) -> Self {
        match self {
            LinExpr::Var(conds) => LinExpr::Var(conds.deep_clone_impl(cur_lin_ty, var_map)),
            LinExpr::TypeVar(types) => LinExpr::TypeVar(types.clone()),
            LinExpr::Const(val) => LinExpr::Const(*val),
            LinExpr::ConstPrim(sym) => LinExpr::ConstPrim(sym.clone()),
            LinExpr::Expr { name, params } => {
                LinExpr::Expr { 
                    name: name.clone(), 
                    params: params.deep_clone_impl(cur_lin_ty, var_map),
                }
            },
            LinExpr::Ident(idx) => LinExpr::Ident(idx.deep_clone_impl(cur_lin_ty, var_map)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinRule {
    pub lhs: LinVec<LinExpr>,
    pub rhs: LinVec<LinExpr>,
    pub is_lower: bool,
}

impl LinRule {
    pub fn new(lhs: LinVec<LinExpr>, rhs: LinVec<LinExpr>, is_lower: bool) -> Self {
        Self {
            lhs,
            rhs,
            is_lower,
        }
    }

    pub fn deep_clone(&self) -> Self {
        let mut var_map = HashMap::new();
        let new_lhs = self.lhs.deep_clone_impl(LinType::LHS, &mut var_map);
        let new_rhs = self.rhs.deep_clone_impl(LinType::RHS, &mut var_map);
        Self {
            lhs: new_lhs,
            rhs: new_rhs,
            is_lower: self.is_lower,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LinResult {
    pub rule: LinRule,
    pub cond_stmts: LinVec<LinExpr>,
    pub cond_pairs: Vec<(LinExprIdx, LinExprIdx)>,
}

impl LinResult {
    pub fn deep_clone(&self) -> Self {
        let mut var_map = HashMap::new();
        let new_lhs = self.rule.lhs.deep_clone_impl(LinType::LHS, &mut var_map);
        let new_rhs = self.rule.rhs.deep_clone_impl(LinType::RHS, &mut var_map);
        let new_rule = LinRule::new(new_lhs, new_rhs, self.rule.is_lower);
        let new_cond_stmts = self.cond_stmts.deep_clone_impl(LinType::Cond, &mut var_map);
        let new_cond_pairs = self.cond_pairs.iter().map(
            |(x, y)| (
                x.deep_clone_impl(LinType::Cond, &mut var_map), 
                y.deep_clone_impl(LinType::Cond, &mut var_map)
            )
        ).collect();
        Self {
            rule: new_rule,
            cond_stmts: new_cond_stmts,
            cond_pairs: new_cond_pairs,
        }
    }

    pub fn get(&self, stmt_idx: &LinExprIdx) -> Option<&LinExpr> {
        match stmt_idx {
            LinExprIdx::LHS(idx) => self.rule.lhs.get(*idx.borrow()),
            LinExprIdx::RHS(idx) => self.rule.rhs.get(*idx.borrow()),
            LinExprIdx::Cond(idx) => self.cond_stmts.get(*idx.borrow()),
        }
    }

    pub fn get_idx(&self, stmt_idx: &LinExprIdx) -> Option<LinExprIdx> {
        match stmt_idx {
            LinExprIdx::LHS(idx) => Some(LinExprIdx::LHS(self.rule.lhs.get_idx_ref(*idx.borrow())?.clone())),
            LinExprIdx::RHS(idx) => Some(LinExprIdx::RHS(self.rule.rhs.get_idx_ref(*idx.borrow())?.clone())),
            LinExprIdx::Cond(idx) => Some(LinExprIdx::Cond(self.cond_stmts.get_idx_ref(*idx.borrow())?.clone())),
        }
    }

    pub fn get_mut(&mut self, stmt_idx: &LinExprIdx) -> Option<&mut LinExpr> {
        match stmt_idx {
            LinExprIdx::LHS(idx) => self.rule.lhs.get_mut(*idx.borrow()),
            LinExprIdx::RHS(idx) => self.rule.rhs.get_mut(*idx.borrow()),
            LinExprIdx::Cond(idx) => self.cond_stmts.get_mut(*idx.borrow()),
        }
    }

    pub fn replace(&mut self, stmt_idx: &LinExprIdx, expr: LinExpr) {
        match stmt_idx {
            LinExprIdx::LHS(idx) => self.rule.lhs.replace(*idx.borrow(), expr),
            LinExprIdx::RHS(idx) => self.rule.rhs.replace(*idx.borrow(), expr),
            LinExprIdx::Cond(idx) => self.cond_stmts.replace(*idx.borrow(), expr),
        }
    }

    pub fn get_idx_list(&self) -> Vec<LinExprIdx> {
        let mut idx_list = Vec::new();
        idx_list.append(&mut (0..self.rule.lhs.len()).map(
            |x| LinExprIdx::LHS(self.rule.lhs.get_idx_ref(x).unwrap().clone())
        ).collect());
        idx_list.append(&mut (0..self.rule.rhs.len()).map(
            |x| LinExprIdx::RHS(self.rule.rhs.get_idx_ref(x).unwrap().clone())
        ).collect());
        idx_list.append(&mut (0..self.cond_stmts.len()).map(
            |x| LinExprIdx::Cond(self.cond_stmts.get_idx_ref(x).unwrap().clone())
        ).collect());
        idx_list
    }

    pub fn get_predecessors(&self, idx: &LinExprIdx) -> Vec<LinExprIdx> {
        let idx_list = self.get_idx_list();
        let mut predec_vec = Vec::new();

        for cur_idx in idx_list.iter() {
            let stmt = self.get(cur_idx).unwrap();
            match stmt {
                LinExpr::Expr { name: _, params } => {
                    if params.contains(idx) {
                        predec_vec.push(cur_idx.clone());
                    }
                },
                LinExpr::Ident(ident_idx) => {
                    if ident_idx == idx {
                        predec_vec.push(cur_idx.clone());
                    }
                }
                _ => (),
            }
        }

        predec_vec
    }    

    pub fn remove(&mut self, stmt_idx: LinExprIdx) -> LinExpr {
        let (_, expr) = match stmt_idx {
            LinExprIdx::LHS(idx) => self.rule.lhs.remove(*idx.borrow()),
            LinExprIdx::RHS(idx) => self.rule.rhs.remove(*idx.borrow()),
            LinExprIdx::Cond(idx) => self.cond_stmts.remove(*idx.borrow()),
        };
        expr
    }
    
    // returns if the removal happened
    pub fn remove_and_subst(&mut self, target_idx: LinExprIdx, subst_idx: LinExprIdx) -> bool {
        let idx_list = self.get_predecessors(&target_idx);
        if idx_list.len() > 0 {
            for idx in idx_list {
                let stmt = self.get_mut(&idx).unwrap();
                match stmt {
                    LinExpr::Var(conds) => {
                        let mut new_conds = Vec::new();
                        for cond_exprs in conds {
                            let mut new_cond_exprs = Vec::new();
                            for cond_expr in cond_exprs {
                                match cond_expr {
                                    CondExpr::Expr { name, params } => {
                                        let mut new_params = Vec::new();
                                        for param in params {
                                            if param == &target_idx {
                                                new_params.push(subst_idx.clone());
                                            }
                                            else {
                                                new_params.push(param.clone());
                                            }
                                        }
                                        new_cond_exprs.push(CondExpr::Expr { name: name.clone(), params: new_params });
                                    },
                                    CondExpr::Ident(inner_idx) => {
                                        if inner_idx == &target_idx {
                                            new_cond_exprs.push(CondExpr::Ident(subst_idx.clone()));
                                        }
                                    },
                                    _ => new_cond_exprs.push(cond_expr.clone()),
                                }
                            }
                            new_conds.push(new_cond_exprs);
                        }
                    },
                    LinExpr::Expr { name, params } => {
                        let mut new_params = Vec::new();
                        for param in params {
                            if param == &target_idx {
                                new_params.push(subst_idx.clone());
                            }
                            else {
                                new_params.push(param.clone());
                            }
                        }
                        *stmt = LinExpr::Expr { name: name.clone(), params: new_params };
                    },
                    LinExpr::Ident(ident_idx) => {
                        assert!(ident_idx == &target_idx);
                        *stmt = LinExpr::Ident(subst_idx.clone());
                    },
                    _ => (),
                }
            }

            let new_cond_pairs = self.cond_pairs.iter().map(|(idx1, idx2)| {
                let new_idx1 = if idx1 == &target_idx { subst_idx.clone() } else { idx1.clone() };
                let new_idx2 = if idx2 == &target_idx { subst_idx.clone() } else { idx2.clone() };
                (new_idx1, new_idx2)
            }).collect();
            self.cond_pairs = new_cond_pairs;

            self.remove(target_idx);
            true
        }
        else {
            match (&target_idx, &subst_idx) {
                (LinExprIdx::LHS(idx1), LinExprIdx::LHS(idx2)) |
                (LinExprIdx::RHS(idx1), LinExprIdx::RHS(idx2)) |
                (LinExprIdx::Cond(idx1), LinExprIdx::Cond(idx2))
                    if *idx1.borrow() == *idx2.borrow() + 1 => {
                    let new_cond_pairs = self.cond_pairs.iter().map(|(idx1, idx2)| {
                        let new_idx1 = if idx1 == &target_idx { subst_idx.clone() } else { idx1.clone() };
                        let new_idx2 = if idx2 == &target_idx { subst_idx.clone() } else { idx2.clone() };
                        (new_idx1, new_idx2)
                    }).collect();
                    self.cond_pairs = new_cond_pairs;

                    self.remove(target_idx);
                    true
                },
                _ => {
                    *self.get_mut(&target_idx).unwrap() = LinExpr::Ident(subst_idx);
                    false
                }
            }
        }
    }

    pub fn insert(&mut self, idx: &LinExprIdx, val: LinExpr) {
        match idx {
            LinExprIdx::LHS(inner_idx) => self.rule.lhs.insert(*inner_idx.borrow(), val),
            LinExprIdx::RHS(inner_idx) => self.rule.rhs.insert(*inner_idx.borrow(), val),
            LinExprIdx::Cond(inner_idx) => self.cond_stmts.insert(*inner_idx.borrow(), val),
        }
    }

    pub fn insert_pair(&mut self, idx: LinExprIdx, val: LinExpr) {
        match idx {
            LinExprIdx::LHS(inner_idx) => self.rule.lhs.insert_pair(*inner_idx.borrow(), inner_idx.clone(), val),
            LinExprIdx::RHS(inner_idx) => self.rule.rhs.insert_pair(*inner_idx.borrow(), inner_idx.clone(), val),
            LinExprIdx::Cond(inner_idx) => self.cond_stmts.insert_pair(*inner_idx.borrow(), inner_idx.clone(), val),
        }
    }

    pub fn remove_dangling_expr(&mut self, match_stmts: &mut Vec<MatchStmt>) {
        assert!(match_stmts.len() == self.rule.lhs.len());

        // mark all used expressions
        let mut is_used_vec = vec![
            vec![false; self.rule.lhs.len()], 
            vec![false; self.rule.rhs.len()], 
            vec![false; self.cond_stmts.len()]
        ];

        fn mark_expression(lin_result: &LinResult, is_used_vec: &mut Vec<Vec<bool>>, probe_idx: &LinExprIdx) {
            match probe_idx {
                LinExprIdx::LHS(idx) => { is_used_vec[0][*idx.borrow()] = true; },
                LinExprIdx::RHS(idx) => { is_used_vec[1][*idx.borrow()] = true; },
                LinExprIdx::Cond(idx) => { is_used_vec[2][*idx.borrow()] = true; },
            }

            let probe_stmt = lin_result.get(probe_idx).unwrap();
            match probe_stmt {
                LinExpr::Expr { name: _, params } => {
                    for param in params {
                        mark_expression(lin_result, is_used_vec, param);
                    }
                },
                LinExpr::Ident(ident_idx) => mark_expression(lin_result, is_used_vec, ident_idx),
                _ => (),
            }
        }

        let lhs_probe_idx = LinExprIdx::LHS(self.rule.lhs.get_idx_ref(self.rule.lhs.len() - 1).unwrap().clone());
        mark_expression(self, &mut is_used_vec, &lhs_probe_idx);
        let rhs_probe_idx = LinExprIdx::RHS(self.rule.rhs.get_idx_ref(self.rule.rhs.len() - 1).unwrap().clone());
        mark_expression(self, &mut is_used_vec, &rhs_probe_idx);
        for (fst, snd) in &self.cond_pairs {
            mark_expression(self, &mut is_used_vec, fst);
            mark_expression(self, &mut is_used_vec, snd);
        }

        // remove all unmarked expressions
        let lhs_remove_vec = is_used_vec[0].iter().enumerate().filter(|(_, b)| !**b).map(
            |(v, _)| self.rule.lhs.get_idx_ref(v).unwrap().clone()
        ).collect::<Vec<_>>();
        let rhs_remove_vec = is_used_vec[1].iter().enumerate().filter(|(_, b)| !**b).map(
            |(v, _)| self.rule.rhs.get_idx_ref(v).unwrap().clone()
        ).collect::<Vec<_>>();
        let cond_remove_vec = is_used_vec[2].iter().enumerate().filter(|(_, b)| !**b).map(
            |(v, _)| self.cond_stmts.get_idx_ref(v).unwrap().clone()
        ).collect::<Vec<_>>();

        for v in lhs_remove_vec {
            let v_inner = *v.borrow();
            self.rule.lhs.remove(v_inner);
            match_stmts.remove(v_inner);
        }
        for v in rhs_remove_vec { self.rule.rhs.remove(*v.borrow()); }
        for v in cond_remove_vec { self.cond_stmts.remove(*v.borrow()); }
    }

    pub fn remove_redundant_idents(&mut self, match_stmts: &mut Vec<MatchStmt>) {
        assert!(match_stmts.len() == self.rule.lhs.len());

        let mut cur_idx = 0;
        let mut lhs_len = self.rule.lhs.len();
        while cur_idx < lhs_len {
            let cur_lin_idx = LinExprIdx::LHS(self.rule.lhs.get_idx_ref(cur_idx).unwrap().clone());
            let stmt = self.get(&cur_lin_idx).unwrap();
            match stmt {
                LinExpr::Ident(ident_idx) => {
                    // no need to check if any predecessor exists: Ident should not exist in LHS
                    let is_removed = self.remove_and_subst(cur_lin_idx, ident_idx.clone());
                    if is_removed {
                        match_stmts.remove(cur_idx);
                        lhs_len -= 1;
                    }
                    else {
                        cur_idx += 1;
                    }
                },
                _ => {
                    cur_idx += 1;
                },
            }
        }
        assert!(self.rule.lhs.len() == lhs_len);
        assert!(match_stmts.len() == lhs_len);

        let mut rhs_len = self.rule.rhs.len();
        cur_idx = 0;
        while cur_idx < rhs_len {
            let cur_lin_idx = LinExprIdx::RHS(self.rule.rhs.get_idx_ref(cur_idx).unwrap().clone());
            let stmt = self.get(&cur_lin_idx).unwrap();
            match stmt {
                LinExpr::Ident(ident_idx) => {
                    if self.get_predecessors(ident_idx).len() > 0 {
                        cur_idx += 1;
                        continue;
                    }
                    let is_removed = self.remove_and_subst(cur_lin_idx, ident_idx.clone());
                    if is_removed {
                        rhs_len -= 1;
                    }
                    else {
                        cur_idx += 1;
                    }
                },
                _ => {
                    cur_idx += 1;
                },
            }
        }
        assert!(self.rule.rhs.len() == rhs_len);

        let mut conds_len = self.cond_stmts.len();
        cur_idx = 0;
        while cur_idx < conds_len {
            let cur_lin_idx = LinExprIdx::Cond(self.cond_stmts.get_idx_ref(cur_idx).unwrap().clone());
            let stmt = self.get(&cur_lin_idx).unwrap();
            match stmt {
                LinExpr::Ident(ident_idx) => {
                    if self.get_predecessors(ident_idx).len() > 0 {
                        cur_idx += 1;
                        continue;
                    }
                    let is_removed = self.remove_and_subst(cur_lin_idx, ident_idx.clone());
                    if is_removed {
                        conds_len -= 1;
                    }
                    else {
                        cur_idx += 1;
                    }
                },
                _ => {
                    cur_idx += 1;
                },
            }
        }
        assert!(self.cond_stmts.len() == conds_len);
    }
}

// Types
pub fn get_all_types() -> [Type; 34] {
    [
        types::I8, types::I16, types::I32, types::I64, types::I128, 
        types::F32, types::F64, types::R32, types::R64, 
        types::I8X2, types::I8X2XN, types::I8X4, types::I16X2, 
        types::I8X4XN, types::I8X8, types::I16X4, types::I32X2,
        types::F32X2, types::I8X8XN, types::I16X4XN, types::I32X2XN, 
        types::F32X2XN, types::I8X16, types::I16X8, types::I32X4, 
        types::I64X2, types::F32X4, types::F64X2, types::I8X16XN, 
        types::I16X8XN, types::I32X4XN, types::I64X2XN,
        types::F32X4XN, types::F64X2XN,
        // types::I8X32, types::I16X16, types::I32X8, types::I64X4,
        // types::I128X2, types::F32X8, types::F64X4,
        // types::I8X32XN, types::I16X16XN, types::I32X8XN, types::I64X4XN, 
        // types::I128X2XN, types::F32X8XN, types::F64X4XN,
        // types::I8X64, types::I16X32, types::I32X16, types::I64X8, 
        // types::I128X4, types::F32X16, types::F64X8,
        // types::I8X64XN, types::I16X32XN, types::I32X16XN, types::I64X8XN, 
        // types::I128X4XN, types::F32X16XN, types::F64X8XN,
    ]
}

pub fn get_imm128_types() -> [Type; 5] {
    [
        types::I8, types::I16, types::I32, types::I64, types::I128
    ]
}

pub fn get_imm64_types() -> [Type; 4] {
    [
        types::I8, types::I16, types::I32, types::I64
    ]
}

pub fn get_imm32_types() -> [Type; 3] {
    [
        types::I8, types::I16, types::I32
    ]
}

pub fn get_types_intersection(types1: Vec<Type>, types2: Vec<Type>) -> Vec<Type> {
    types1.into_iter().filter(|x| types2.contains(x)).collect()
}

pub type UnifiedStmt = LinExpr;
pub type UnifiedRule = LinRule;
pub type UnifiedResult = LinResult;
pub type UnifiedExprIdx = LinExprIdx;

// Rule conditions
// Only consider Var = Expr(Var, ..) case
#[derive(Clone, Debug, PartialEq)]
pub enum CondExpr {
    Var,
    Const(i128),
    ConstPrim(String),
    Expr { name: String, params: Vec<LinExprIdx> },
    Ident(LinExprIdx),
}

impl DeepClone for CondExpr {
    fn deep_clone_impl(
        &self,
        cur_lin_ty: LinType, 
        var_map: &mut HashMap<(LinType, usize), Rc<RefCell<usize>>>
    ) -> Self {
        match self {
            CondExpr::Expr { name, params } => {
                CondExpr::Expr { name: name.clone(), params: params.deep_clone_impl(cur_lin_ty, var_map) }
            },
            CondExpr::Ident(idx) => CondExpr::Ident(idx.deep_clone_impl(cur_lin_ty, var_map)),
            _ => self.clone(),
        }
    }
}

// Matched expressions
#[derive(Clone, Debug, PartialEq)]
pub enum MatchStmt {
    Expr { data: IRData, instrs: Vec<Instruction> }, // for link info, refer to LinExpr
    Arg(Vec<Vec<CondExpr>>),
    Const(i128),
    Nil, // due to typevar and conds
    None, // identifier for expressions not matched
}

impl DeepClone for MatchStmt {
    fn deep_clone_impl(
        &self,
        cur_lin_ty: LinType, 
        var_map: &mut HashMap<(LinType, usize), Rc<RefCell<usize>>>
    ) -> Self {
        assert!(cur_lin_ty == LinType::Cond);
        match self {
            MatchStmt::Arg(conds) => {
                MatchStmt::Arg(conds.deep_clone_impl(cur_lin_ty, var_map))
            },
            _ => self.clone(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct MatchResult {
    pub lhs: LinVec<LinExpr>, // UnifiedExpr
    pub rhs: LinVec<LinExpr>,
    pub stmts: Vec<MatchStmt>,

    pub from_learned: bool, // if this is derived from learned rules
    reversed: bool, // if reversed, stmts refer to rhs / if not, lhs
}

impl MatchResult {
    pub fn new(rule: LinRule) -> Self {
        let stmts = vec![MatchStmt::None; rule.lhs.len()];
        Self {
            lhs: rule.lhs,
            rhs: rule.rhs,
            stmts,
            from_learned: false,
            reversed: false,
        }
    }

    pub fn new_with_stmts(rule: LinRule, stmts: Vec<MatchStmt>) -> Self {
        assert!(stmts.len() == rule.lhs.len());
        Self {
            lhs: rule.lhs,
            rhs: rule.rhs,
            stmts,
            from_learned: false,
            reversed: false,
        }
    }

    // new with reversed (used in wasm_norm)
    pub fn new_rev(rule: LinRule, stmts: Vec<MatchStmt>) -> Self {
        assert!(stmts.len() == rule.rhs.len());
        Self {
            lhs: rule.lhs,
            rhs: rule.rhs,
            stmts,
            from_learned: false,
            reversed: true,
        }
    }

    pub fn get(&self, stmt_idx: &LinExprIdx) -> Option<&LinExpr> {
        match stmt_idx {
            LinExprIdx::LHS(idx) => self.lhs.get(*idx.borrow()),
            LinExprIdx::RHS(idx) => self.rhs.get(*idx.borrow()),
            LinExprIdx::Cond(idx) => unreachable!(),
        }
    }

    pub fn deep_clone(&self) -> Self {
        let mut var_map = HashMap::new();
        let new_lhs = self.lhs.deep_clone_impl(LinType::LHS, &mut var_map);
        let new_rhs = self.rhs.deep_clone_impl(LinType::RHS, &mut var_map);
        let new_stmts = self.stmts.deep_clone_impl(LinType::Cond, &mut var_map); // LinType actually not used
        Self {
            lhs: new_lhs,
            rhs: new_rhs,
            stmts: new_stmts,
            from_learned: self.from_learned,
            reversed: self.reversed,
        }
    }

    pub fn is_reversed(&self) -> bool {
        self.reversed
    }

    pub fn len(&self) -> usize {
        assert!((self.reversed && (self.rhs.len() == self.stmts.len())) ||
                (!self.reversed && (self.lhs.len() == self.stmts.len())));
        self.stmts.len()
    }

    pub fn get_pair(self) -> (LinRule, Vec<MatchStmt>) {
        (LinRule { lhs: self.lhs, rhs: self.rhs, is_lower: false }, self.stmts) // default of is_lower is false
    }

    // get name from end of lhs (use with caution!)
    pub fn get_name(&self) -> String {
        match self.lhs.get(self.lhs.len() - 1).unwrap() {
            LinExpr::Expr { name, .. } => name.clone(),
            _ => unreachable!(),
        }
    }

    pub fn reverse(self) -> Self {
        assert!(!self.reversed);
        assert!(self.stmts.len() == self.lhs.len());

        // Step 1: destruct all
        let (lhs_store, lhs_idx) = self.lhs.destruct();
        let lhs_stmts = self.stmts;
        assert!(lhs_store.len() == lhs_stmts.len());
        let (rhs_store, rhs_idx) = self.rhs.destruct();

        // Step 2: get RHS slice
        let mut lhs_idx_list = vec![None; lhs_store.len()];
        let mut rhs_idx_list = vec![None; rhs_store.len()];
        let mut new_lhs_pairs = Vec::new();
        let mut new_rhs_pairs = Vec::new();
        
        fn traverse_expressions(
            lhs_idx_list: &mut Vec<Option<LinExprIdx>>, rhs_idx_list: &mut Vec<Option<LinExprIdx>>,
            new_pairs: &mut Vec<(LinExpr, Rc<RefCell<usize>>, MatchStmt)>,
            lhs_store: &Vec<LinExpr>, lhs_idx: &Vec<Rc<RefCell<usize>>>, lhs_stmts: &Vec<MatchStmt>,
            rhs_store: &Vec<LinExpr>, rhs_idx: &Vec<Rc<RefCell<usize>>>, 
            probe_type: LinType, probe_idx: (LinType, usize)
        ) {
            let (cur_store, cur_idx, cur_stmt) = match probe_idx.0 {
                LinType::LHS => {
                    let store = lhs_store[probe_idx.1].clone();
                    let idx = lhs_idx[probe_idx.1].clone();
                    let stmt = lhs_stmts[probe_idx.1].clone();
                    (store, idx, stmt)
                },
                LinType::RHS => {
                    let store = rhs_store[probe_idx.1].clone();
                    let idx = rhs_idx[probe_idx.1].clone();
                    let stmt = MatchStmt::None;
                    (store, idx, stmt)
                },
                LinType::Cond => unreachable!(),
            };

            // set idx_list vectors
            match probe_type {
                LinType::LHS => {
                    match probe_idx.0 {
                        LinType::LHS => { lhs_idx_list[probe_idx.1] = Some(LinExprIdx::LHS(cur_idx.clone())); },
                        LinType::RHS => { rhs_idx_list[probe_idx.1] = Some(LinExprIdx::LHS(cur_idx.clone())); },
                        LinType::Cond => unreachable!(),
                    }
                },
                LinType::RHS => {
                    match probe_idx.0 {
                        LinType::LHS => { 
                            if lhs_idx_list[probe_idx.1].is_none() {
                                lhs_idx_list[probe_idx.1] = Some(LinExprIdx::RHS(cur_idx.clone())); 
                            }
                        },
                        LinType::RHS => { 
                            if rhs_idx_list[probe_idx.1].is_none() {
                                rhs_idx_list[probe_idx.1] = Some(LinExprIdx::RHS(cur_idx.clone())); 
                            }
                        },
                        LinType::Cond => unreachable!(),
                    }
                },
                LinType::Cond => unreachable!(),
            }
            
            // adjust parameter indices
            match &cur_store {
                LinExpr::Expr { name, params } => {
                    let mut new_params = Vec::new();
                    for param in params {
                        match param {
                            LinExprIdx::LHS(idx) => {
                                let idx_borrowed = *idx.borrow();
                                if lhs_idx_list[idx_borrowed].is_none() {
                                    traverse_expressions(
                                        lhs_idx_list, rhs_idx_list,
                                        new_pairs,
                                        lhs_store, lhs_idx, lhs_stmts,
                                        rhs_store, rhs_idx,
                                        probe_type, (LinType::LHS, idx_borrowed)
                                    );
                                }

                                assert!(lhs_idx_list[idx_borrowed].is_some());

                                match &lhs_idx_list[idx_borrowed] {
                                    Some(x) => new_params.push(x.clone()),
                                    None => unreachable!(),
                                }
                            },
                            LinExprIdx::RHS(idx) => {
                                let idx_borrowed = *idx.borrow();
                                if rhs_idx_list[idx_borrowed].is_none() {
                                    traverse_expressions(
                                        lhs_idx_list, rhs_idx_list,
                                        new_pairs,
                                        lhs_store, lhs_idx, lhs_stmts,
                                        rhs_store, rhs_idx,
                                        probe_type, (LinType::RHS, idx_borrowed)
                                    )
                                }

                                assert!(rhs_idx_list[idx_borrowed].is_some());
                                
                                match &rhs_idx_list[idx_borrowed] {
                                    Some(x) => new_params.push(x.clone()),
                                    None => unreachable!(),
                                }
                            },
                            LinExprIdx::Cond(_) => unreachable!(),
                        }
                    }
                    assert!(params.len() == new_params.len());

                    let new_store = LinExpr::Expr { name: name.clone(), params: new_params };
                    new_pairs.push((new_store, cur_idx, cur_stmt));
                },
                LinExpr::Ident(idx) => {
                    match idx {
                        LinExprIdx::LHS(inner_idx) => {
                            let idx_borrowed = *inner_idx.borrow();
                            if lhs_idx_list[idx_borrowed].is_none() {
                                traverse_expressions(
                                    lhs_idx_list, rhs_idx_list,
                                    new_pairs,
                                    lhs_store, lhs_idx, lhs_stmts,
                                    rhs_store, rhs_idx,
                                    probe_type, (LinType::LHS, idx_borrowed)
                                );

                                // overwrite idx list to the inner idx
                                match probe_idx.0 {
                                    LinType::LHS => { lhs_idx_list[probe_idx.1] = lhs_idx_list[idx_borrowed].clone(); },
                                    LinType::RHS => { rhs_idx_list[probe_idx.1] = lhs_idx_list[idx_borrowed].clone(); },
                                    LinType::Cond => unreachable!(),
                                }
                            }

                            assert!(lhs_idx_list[idx_borrowed].is_some());
                        },
                        LinExprIdx::RHS(inner_idx) => {
                            let idx_borrowed = *inner_idx.borrow();
                            if rhs_idx_list[idx_borrowed].is_none() {
                                traverse_expressions(
                                    lhs_idx_list, rhs_idx_list,
                                    new_pairs,
                                    lhs_store, lhs_idx, lhs_stmts,
                                    rhs_store, rhs_idx,
                                    probe_type, (LinType::RHS, idx_borrowed)
                                );

                                // overwrite idx list to the inner idx
                                match probe_idx.0 {
                                    LinType::LHS => { lhs_idx_list[probe_idx.1] = rhs_idx_list[idx_borrowed].clone(); },
                                    LinType::RHS => { rhs_idx_list[probe_idx.1] = rhs_idx_list[idx_borrowed].clone(); },
                                    LinType::Cond => unreachable!(),
                                }
                            }

                            assert!(rhs_idx_list[idx_borrowed].is_some());
                        },
                        LinExprIdx::Cond(_) => unreachable!(),
                    }
                },
                _ => new_pairs.push((cur_store, cur_idx.clone(), cur_stmt)),
            }
        }

        traverse_expressions(
            &mut lhs_idx_list, &mut rhs_idx_list,
            &mut new_lhs_pairs,
            &lhs_store, &lhs_idx, &lhs_stmts,
            &rhs_store, &rhs_idx,
            LinType::LHS, (LinType::RHS, rhs_store.len() - 1)
        );
        traverse_expressions(
            &mut lhs_idx_list, &mut rhs_idx_list,
            &mut new_rhs_pairs,
            &lhs_store, &lhs_idx, &lhs_stmts,
            &rhs_store, &rhs_idx,
            LinType::RHS, (LinType::LHS, lhs_store.len() - 1)
        );

        // unzip pairs
        let mut new_lhs_store = Vec::new();
        let mut new_lhs_idx = Vec::new();
        for (store, idx, _) in new_lhs_pairs {
            new_lhs_store.push(store);
            new_lhs_idx.push(idx);
        }

        let mut new_rhs_store = Vec::new();
        let mut new_rhs_idx = Vec::new();
        let mut new_stmts = Vec::new();
        for (store, idx, stmt) in new_rhs_pairs {
            assert!(stmt != MatchStmt::None);
            new_rhs_store.push(store);
            new_rhs_idx.push(idx);
            new_stmts.push(stmt);
        }

        let new_lhs_vec = LinVec::construct(new_lhs_store, new_lhs_idx);
        let new_rhs_vec = LinVec::construct(new_rhs_store, new_rhs_idx);
        let new_rule = LinRule::new(new_lhs_vec, new_rhs_vec, false);
        
        let result = Self::new_rev(new_rule, new_stmts);
        result
    }
}
