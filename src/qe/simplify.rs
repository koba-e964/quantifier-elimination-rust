use crate::formula::{Formula, Relation};
use num_traits::Zero;
use std::collections::BTreeMap;

/// Simplify Boolean structure and evaluate variable-free polynomial atoms.
pub fn simplify(formula: &Formula) -> Formula {
    match formula {
        Formula::True | Formula::False => formula.clone(),
        Formula::Atom(atom) if atom.polynomial.variables().next().is_none() => {
            let value = atom.polynomial.evaluate(&BTreeMap::new());
            let zero = num_rational::BigRational::zero();
            let result = match atom.relation {
                Relation::Equal => value == zero,
                Relation::NotEqual => value != zero,
                Relation::Less => value < zero,
                Relation::LessOrEqual => value <= zero,
                Relation::Greater => value > zero,
                Relation::GreaterOrEqual => value >= zero,
            };
            if result {
                Formula::True
            } else {
                Formula::False
            }
        }
        Formula::Atom(_) => formula.clone(),
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
