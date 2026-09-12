use crate::formula::Formula;

/// Simplify Boolean structure without changing polynomial atoms.
pub fn simplify(formula: &Formula) -> Formula {
    match formula {
        Formula::True | Formula::False | Formula::Atom(_) => formula.clone(),
        Formula::Not(body) => match simplify(body) {
            Formula::True => Formula::False,
            Formula::False => Formula::True,
            Formula::Not(inner) => *inner,
            simplified => Formula::Not(Box::new(simplified)),
        },
        Formula::And(formulas) => simplify_conjunction(formulas),
        Formula::Or(formulas) => simplify_disjunction(formulas),
        Formula::Quantified {
            quantifier,
            variable,
            body,
        } => Formula::Quantified {
            quantifier: *quantifier,
            variable: *variable,
            body: Box::new(simplify(body)),
        },
    }
}

fn simplify_conjunction(formulas: &[Formula]) -> Formula {
    let mut simplified = Vec::new();
    for formula in formulas {
        match simplify(formula) {
            Formula::True => {}
            Formula::False => return Formula::False,
            Formula::And(nested) => simplified.extend(nested),
            formula => simplified.push(formula),
        }
    }
    match simplified.len() {
        0 => Formula::True,
        1 => simplified.pop().unwrap(),
        _ => Formula::And(simplified),
    }
}

fn simplify_disjunction(formulas: &[Formula]) -> Formula {
    let mut simplified = Vec::new();
    for formula in formulas {
        match simplify(formula) {
            Formula::False => {}
            Formula::True => return Formula::True,
            Formula::Or(nested) => simplified.extend(nested),
            formula => simplified.push(formula),
        }
    }
    match simplified.len() {
        0 => Formula::False,
        1 => simplified.pop().unwrap(),
        _ => Formula::Or(simplified),
    }
}
