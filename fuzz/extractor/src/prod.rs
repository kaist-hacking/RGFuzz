use itertools::Itertools;
use wasm_ast::{Instruction, ParametricInstruction, ResultType, ValueType};

use crate::wasm_comp::get_instruction_name;

fn type_to_string(ty: &ValueType) -> String {
    match ty {
        ValueType::I32 => String::from("\"i32\""),
        ValueType::I64 => String::from("\"i64\""),
        ValueType::F32 => String::from("\"f32\""),
        ValueType::F64 => String::from("\"f64\""),
        ValueType::V128 => String::from("\"v128\""),
        ValueType::FunctionReference => String::from("\"funcref\""),
        ValueType::ExternalReference => String::from("\"externref\""),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProdToken {
    Instr(Instruction, Option<ProdOperand>),
    Arg(usize),
}

impl ToString for ProdToken {
    fn to_string(&self) -> String {
        let mut result_str = String::new();
        match self {
            ProdToken::Instr(
                instr @ Instruction::Parametric(ParametricInstruction::Select(Some(types))), 
                operand
            ) => {
                assert!(operand.is_none());
                result_str += "Instr(";
                let instr_name = get_instruction_name(instr);
                result_str += "\"";
                result_str += instr_name.as_str();
                result_str += "\"";
                result_str += ",([";
                result_str += types.iter().map(type_to_string).join(",").as_str();
                result_str += "],))";
            }
            ProdToken::Instr(instr, operand) => {
                result_str += "Instr(";
                let instr_name = get_instruction_name(instr);
                result_str += "\"";
                result_str += instr_name.as_str();
                result_str += "\"";
                match operand {
                    Some(x) => {
                        result_str += ",(";
                        result_str += x.to_string().as_str();
                        result_str += ",)"
                    },
                    None => {
                        result_str += ",()"
                    }
                }
                result_str += ")";
            },
            ProdToken::Arg(varid) => {
                result_str += "Instr(";
                result_str += "\"arg\"";
                result_str += ",(";
                result_str += (*varid).to_string().as_str();
                result_str += ",))";
            },
        }
        result_str
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProdOperand {
    OpConst(i128),
    OpArg(usize),
}

impl ToString for ProdOperand {
    fn to_string(&self) -> String {
        let mut result_str = String::new();
        match self {
            ProdOperand::OpConst(val) => result_str += (*val).to_string().as_str(),
            ProdOperand::OpArg(varid) => {
                result_str += "(";
                result_str += "\"oparg\"";
                result_str += ",";
                result_str += (*varid).to_string().as_str();
                result_str += ")";
            },
        }
        result_str
    }
}

pub type ProdInstr = Vec<ProdToken>;

// tree structure
#[derive(Clone, Debug, PartialEq)]
pub enum ProdCondExpr {
    Wildcard,
    Var(usize),
    Const(i128),
    ConstPrim(String),
    Expr { name: String, params: Vec<ProdCondExpr> },
}

impl ToString for ProdCondExpr {
    fn to_string(&self) -> String {
        let mut result_str = String::new();
        match self {
            ProdCondExpr::Wildcard => {
                result_str += "\"*\"";
            },
            ProdCondExpr::Var(idx) => {
                result_str += "\"var";
                result_str += idx.to_string().as_str();
                result_str += "\"";
            },
            ProdCondExpr::Const(val) => {
                result_str += (*val).to_string().as_str();
            },
            ProdCondExpr::ConstPrim(sym) => {
                result_str += "\"";
                result_str += sym.as_str();
                result_str += "\"";
            },
            ProdCondExpr::Expr { name, params } => {
                result_str += "(\"";
                result_str += name.as_str();
                result_str += "\",";
                result_str += params.iter().map(|x| x.to_string()).join(",").as_str();
                result_str += ")";
            },
        }
        result_str
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProdCondition {
    pub arg: usize,
    pub conds: Vec<ProdCondExpr>,
}

impl ToString for ProdCondition {
    fn to_string(&self) -> String {
        let mut result_str = String::new();
        result_str += "(";
        result_str += self.arg.to_string().as_str();
        result_str += ",";
        result_str += "[";
        result_str += self.conds.iter().map(|x| x.to_string()).join(",").as_str();
        result_str += "])";
        result_str
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProdRule {
    pub param_types: ResultType,
    pub ret_types: ResultType,
    pub instrs: ProdInstr,
    pub arg_conds: Vec<ProdCondition>,
    pub oparg_conds: Vec<ProdCondition>,
}

impl ToString for ProdRule {
    fn to_string(&self) -> String {
        let mut param_str = self.param_types.kinds().iter().map(type_to_string);
        let mut ret_str = self.ret_types.kinds().iter().map(type_to_string);

        let mut result_str = String::new();
        result_str += "([";
        result_str += param_str.join(",").as_str();
        result_str += "],[";
        result_str += ret_str.join(",").as_str();
        result_str += "],[";
        result_str += self.instrs.iter().map(|x| x.to_string()).join(",").as_str();
        result_str += "],[";
        result_str += self.arg_conds.iter().map(|x| x.to_string()).join(",").as_str();
        result_str += "],[";
        result_str += self.oparg_conds.iter().map(|x| x.to_string()).join(",").as_str();
        result_str += "])";
        result_str
    }
}