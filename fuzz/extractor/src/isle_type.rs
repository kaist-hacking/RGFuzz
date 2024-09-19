// Step 1.4: Type linearized ISLE rules

use crate::{norm::LinResult, isle::ISLEParseOptions, isle_lin::linearize_rules_opt, isle_inl::process_internals};

fn type_rule(lin_result: LinResult) -> Vec<LinResult> {
    // Basic separation of typevars from vars are already done in linearization
    // From internals, specialize types of these typevars

    // Handle compiler internals
    let inl_results = process_internals(lin_result);

    inl_results
}

pub fn type_rules_opt(opt: ISLEParseOptions) -> Vec<LinResult> {
    let lin_rules = linearize_rules_opt(opt);
    lin_rules.into_iter().flat_map(type_rule).collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_type_rules() {
        let rules = type_rules_opt(ISLEParseOptions::Lower);
        println!("{:#?}", rules);
    }

    #[test]
    fn test_type_rule_one() {
        let rules = linearize_rules_opt(ISLEParseOptions::Opt);
        println!("{:#?}", type_rule(rules[89].clone()));
    }
}