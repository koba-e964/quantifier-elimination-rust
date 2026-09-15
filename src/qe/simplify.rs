use super::boolean::{
    merge_exact_sign_bounds, merge_sign_constraints, simplify_disjunction,
    simplify_monomial_zero_atom,
};
use super::linear::{
    linear_conjunction_is_infeasible, simplify_redundant_linear_constraints,
    simplify_univariate_linear_bounds,
};
use crate::formula::{Formula, Relation};
use crate::polynomial::Polynomial;
use num_traits::{Signed, Zero};
use std::collections::BTreeMap;

/// Simplify Boolean structure and evaluate variable-free polynomial atoms.
pub fn simplify(formula: &Formula) -> Formula {
    match formula {
        Formula::True | Formula::False => formula.clone(),
        Formula::Atom(atom) => {
            let (polynomial, relation) = canonicalize_atom(&atom.polynomial, atom.relation);
            if let Some(reduced) = simplify_monomial_zero_atom(&polynomial, relation) {
                return reduced;
            }
            if polynomial.variables().next().is_some() {
                return Formula::atom(polynomial, relation);
            }
            let value = polynomial.evaluate(&BTreeMap::new());
            let zero = num_rational::BigRational::zero();
            let result = match relation {
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
        Formula::Not(body) => match simplify(body) {
            Formula::True => Formula::False,
            Formula::False => Formula::True,
            Formula::Not(inner) => *inner,
            Formula::Atom(atom) => Formula::atom(
                atom.polynomial,
                match atom.relation {
                    Relation::Equal => Relation::NotEqual,
                    Relation::NotEqual => Relation::Equal,
                    Relation::Less => Relation::GreaterOrEqual,
                    Relation::LessOrEqual => Relation::Greater,
                    Relation::Greater => Relation::LessOrEqual,
                    Relation::GreaterOrEqual => Relation::Less,
                },
            ),
            Formula::Or(formulas) => simplify(&Formula::And(
                formulas
                    .into_iter()
                    .map(|formula| Formula::Not(Box::new(formula)))
                    .collect(),
            )),
            Formula::And(formulas) => simplify(&Formula::Or(
                formulas
                    .into_iter()
                    .map(|formula| Formula::Not(Box::new(formula)))
                    .collect(),
            )),
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

fn canonicalize_atom(polynomial: &Polynomial, relation: Relation) -> (Polynomial, Relation) {
    let polynomial = polynomial.primitive_part();
    let Some((_, coefficient)) = polynomial.terms().next() else {
        return (polynomial, relation);
    };
    if coefficient.is_negative() {
        (-(polynomial), negate_relation(relation))
    } else {
        (polynomial, relation)
    }
}

fn negate_relation(relation: Relation) -> Relation {
    match relation {
        Relation::Equal => Relation::Equal,
        Relation::NotEqual => Relation::NotEqual,
        Relation::Less => Relation::Greater,
        Relation::LessOrEqual => Relation::GreaterOrEqual,
        Relation::Greater => Relation::Less,
        Relation::GreaterOrEqual => Relation::LessOrEqual,
    }
}

pub(super) fn simplify_conjunction(formulas: &[Formula]) -> Formula {
    let mut simplified = Vec::new();
    for formula in formulas {
        match simplify(formula) {
            Formula::True => {}
            Formula::False => return Formula::False,
            Formula::And(nested) => simplified.extend(nested),
            formula if !simplified.contains(&formula) => simplified.push(formula),
            _ => {}
        }
    }
    if merge_sign_constraints(&mut simplified) {
        return Formula::False;
    }
    if simplify_univariate_linear_bounds(&mut simplified) {
        return Formula::False;
    }
    if linear_conjunction_is_infeasible(&simplified) {
        return Formula::False;
    }
    simplify_redundant_linear_constraints(&mut simplified);
    simplified = merge_exact_sign_bounds(simplified);
    match simplified.len() {
        0 => Formula::True,
        1 => simplified.pop().unwrap(),
        _ => Formula::And(simplified),
    }
}
