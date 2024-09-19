use std::collections::HashSet;
use std::env;

use enum_iterator::all;
use prod::ProdRule;
use prod_typing::get_typing_rule_prods;
use prod_extract::learn_prods;
use rule_match::MatchOption;
use wasm_ast::ValueType;

mod isle;
mod isle_inl;
mod isle_norm;
mod isle_lin;
mod isle_type;
mod isle_subst;
mod isle_cond;
mod wasm_map;
mod wasm_comp;
mod wasm_norm;
mod prod;
mod prod_extract;
mod prod_typing;
mod norm;
mod rule_match;

pub fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return;
    }

    // typing rules
    let mut typing_rules: Vec<ProdRule> = Vec::new();
    for ty in all::<ValueType>() {
        let mut ty_rules = get_typing_rule_prods(vec![ty].into());
        typing_rules.append(&mut ty_rules);
    }
    let mut noret_rules = get_typing_rule_prods(Vec::new().into());
    typing_rules.append(&mut noret_rules);

    // print rules
    if args[1] == "all" {
        let prod_rules = learn_prods(MatchOption::All); // may contain duplicates
        let mut prod_rules_set: HashSet<String> = HashSet::from_iter(prod_rules.iter().map(|x| x.to_string()));
        prod_rules_set.extend(typing_rules.iter().map(|x| x.to_string()));
        for rule in prod_rules_set {
            println!("{}", rule);
        }
    }
    else if args[1] == "opt" {
        let prod_rules = learn_prods(MatchOption::Opt); // may contain duplicates
        let prod_rules_set: HashSet<String> = HashSet::from_iter(prod_rules.iter().map(|x| x.to_string()));
        for rule in prod_rules_set {
            println!("{}", rule);
        }
    }
    else if args[1] == "lower" {
        let prod_rules = learn_prods(MatchOption::Lower); // may contain duplicates
        let prod_rules_set: HashSet<String> = HashSet::from_iter(prod_rules.iter().map(|x| x.to_string()));
        for rule in prod_rules_set {
            println!("{}", rule);
        }
    }
    else if args[1] == "testopt" {
        let prod_rules = learn_prods(MatchOption::TestOpt); // may contain duplicates
        let prod_rules_set: HashSet<String> = HashSet::from_iter(prod_rules.iter().map(|x| x.to_string()));
        for rule in prod_rules_set {
            println!("{}", rule);
        }
    }
    else if args[1] == "testlower" {
        let prod_rules = learn_prods(MatchOption::TestLower); // may contain duplicates
        let prod_rules_set: HashSet<String> = HashSet::from_iter(prod_rules.iter().map(|x| x.to_string()));
        for rule in prod_rules_set {
            println!("{}", rule);
        }
    }
    else if args[1] == "typing" {
        for rule in typing_rules {
            println!("{}", rule.to_string());
        }
    }
    else if args[1] == "optlower" {
        let prod_rules = learn_prods(MatchOption::All); // may contain duplicates
        let prod_rules_set: HashSet<String> = HashSet::from_iter(prod_rules.iter().map(|x| x.to_string()));
        for rule in prod_rules_set {
            println!("{}", rule);
        }
    }
    else if args[1] == "opttyping" {
        let prod_rules = learn_prods(MatchOption::Opt); // may contain duplicates
        let mut prod_rules_set: HashSet<String> = HashSet::from_iter(prod_rules.iter().map(|x| x.to_string()));
        prod_rules_set.extend(typing_rules.iter().map(|x| x.to_string()));
        for rule in prod_rules_set {
            println!("{}", rule);
        }
    }
    else if args[1] == "lowertyping" {
        let prod_rules = learn_prods(MatchOption::Lower); // may contain duplicates
        let mut prod_rules_set: HashSet<String> = HashSet::from_iter(prod_rules.iter().map(|x| x.to_string()));
        prod_rules_set.extend(typing_rules.iter().map(|x| x.to_string()));
        for rule in prod_rules_set {
            println!("{}", rule);
        }
    }
    else {
        //
    }
}