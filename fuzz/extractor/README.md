# Rule Extractor

In this readme, we will explain how the rule extractor is implemented and how to customize it.

## Files

Initially, the rule extractor processes the WebAssembly instructions and ISLE rules to match them into production rules. In this context, the rule extractor has the following files:

- Step 1: ISLE parsing
    - Step 1.1: Extracting and parsing ISLE rules (`src/isle.rs`)
    - Step 1.2: Normalize parsed ISLE rules (`src/isle_norm.rs`)
    - Step 1.3: Linearize normalized ISLE rules (`src/isle_lin.rs`)
    - Step 1.4: Type linearized ISLE rules. Mainly, process directives and rule conditions (`src/isle_type.rs`)
        - Step 1.4.1: Process directives (`src/isle_inl.rs`)
        - Step 1.4.2: Process rule conditions (`src/isle_cond.rs`)
    - Step 1.5: Rule-level substitution for non-optimization and non-lowering rules (`src/isle_subst.rs`)
- Step 2: Instruction-level inference (IR --> WebAssembly instructions)
    - Step 2.1: Map each WebAssembly instruction to Cranelift IR (`src/wasm_map.rs`)
    - Step 2.2: Convert IRs into normalized and linearized ISLE rules (`src/wasm_norm.rs`)
- Step 3: Recursive substitution: rule-level substitution (`src/rule_match.rs`)
- Step 4: Process substituted rules into production rules (`src/prod_extract.rs`)

There are also auxiliary files that defines required data structures and functions:

- `src/wasm_comp.rs`: Defines the typing rules of WebAssembly instructions
- `src/norm.rs`: Defines the data structures used in the extractor
- `src/prod.rs`: Defines the data structures for production rules

## Linearized ISLE Rules

The linearized ISLE rules represent the ISLE rules in a linear form. We designed the rules to be linear to simplify the matching process. To understand the linearized ISLE rules, you can simply think of them as graphs that are represented in a linear form, having the node indices as the reference to the list of indices.

The data structure is defined as the following:

```rs
#[derive(Clone, Debug, PartialEq)]
pub enum LinExpr {
    Var(Vec<Vec<CondExpr>>), // list of rule conditions
    TypeVar(Vec<Type>), // list of possible types
    Const(i128),
    ConstPrim(String),
    Expr { name: String, params: Vec<LinExprIdx> },
    Ident(LinExprIdx), // RHS and Cond may need simple reference (identity) to other expression
}

#[derive(Clone, Debug, PartialEq)]
pub enum LinExprIdx {
    LHS(Rc<RefCell<usize>>),
    RHS(Rc<RefCell<usize>>),
    Cond(Rc<RefCell<usize>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinVec<T> {
    store: Vec<T>,
    idx: Vec<Rc<RefCell<usize>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinRule {
    pub lhs: LinVec<LinExpr>,
    pub rhs: LinVec<LinExpr>,
    pub is_lower: bool,
}
```

Linearized ISLE rules are represented as `LinRule`. Each `LinRule` has a left-hand side (`lhs`) and a right-hand side (`rhs`). These sides are represented as `LinVec<LinExpr>`, where `LinVec` represented a linearized form of graphs. `store` variables store the actual expressions (`LinExpr`), which can have indices to other expressions, in a form of `LinExprIdx`. `LinExprIdx` then references an index to the `idx` variable, which is a list of indices.

The reason that we use `Rc<RefCell<usize>>` for indices is not to update all the indices whenever we insert or remove an expression. This way, to update the indices, we can simply update the `idx` variable.

## Updating the Rule Extractor to New Wasmtime Version

If you want to upgrade the wasmtime version, you will need to write handers for new compiler directives. That is, you need to modify `src/isle_inl.rs` to handle new compiler directives. You can write handlers in a form of `foo(lin_result: &LinResult, cur_idx: &LinExprIdx) -> Option<Vec<LinResult>>`, where `LinResult` is a linearized ISLE rule with conditions defined separately. After you write handlers, you need to register them in `process_internals_one` function with `or_else` function.

The default strategy would be returning `None`, meaning that the rule does not contain the directive the handler focuses on. If the rule contains the directive, the handler should return a linearized ISLE rules after processing the directive. If the directive can be processed in multiple ways, the handler may return a list of all possible result rules.
