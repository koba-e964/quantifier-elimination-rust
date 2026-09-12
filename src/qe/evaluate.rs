use crate::algebra::univariate::UnivariatePolynomial;
use crate::cad::lifting::{decompose_univariate, FormulaEvaluationError};
use crate::formula::{Atom, Formula, Quantifier};
use crate::polynomial::Monomial;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantifierEvaluationError {
    Formula(FormulaEvaluationError),
    NestedQuantifier,
    NonUnivariatePolynomial,
    WrongVariable,
}

impl From<FormulaEvaluationError> for QuantifierEvaluationError {
    fn from(error: FormulaEvaluationError) -> Self {
        Self::Formula(error)
    }
}

/// Decide a formula whose outermost quantifier binds the sole variable `0`.
/// This is the evaluation half of one-variable quantifier elimination; formula
/// synthesis is kept separate for the multivariate CAD implementation.
pub fn decide_univariate(formula: &Formula) -> Result<bool, QuantifierEvaluationError> {
    let (quantifier, variable, body) = match formula {
        Formula::Quantified {
            quantifier,
            variable,
            body,
        } => (*quantifier, *variable, body.as_ref()),
        _ => return Err(QuantifierEvaluationError::WrongVariable),
    };
    if variable != 0 {
        return Err(QuantifierEvaluationError::WrongVariable);
    }
    let mut polynomials = Vec::new();
    collect_polynomials(body, &mut polynomials)?;
    let cells = decompose_univariate(&polynomials);
    let values = cells
        .iter()
        .map(|cell| cell.evaluate_formula(body))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(match quantifier {
        Quantifier::Exists => values.into_iter().any(|value| value),
        Quantifier::Forall => values.into_iter().all(|value| value),
    })
}

fn collect_polynomials(
    formula: &Formula,
    polynomials: &mut Vec<UnivariatePolynomial>,
) -> Result<(), QuantifierEvaluationError> {
    match formula {
        Formula::True | Formula::False => Ok(()),
        Formula::Atom(atom) => {
            polynomials.push(to_univariate(atom)?);
            Ok(())
        }
        Formula::Not(body) => collect_polynomials(body, polynomials),
        Formula::And(formulas) | Formula::Or(formulas) => formulas
            .iter()
            .try_for_each(|formula| collect_polynomials(formula, polynomials)),
        Formula::Quantified { .. } => Err(QuantifierEvaluationError::NestedQuantifier),
    }
}

fn to_univariate(atom: &Atom) -> Result<UnivariatePolynomial, QuantifierEvaluationError> {
    if atom.polynomial.variables().any(|variable| variable != 0) {
        return Err(QuantifierEvaluationError::NonUnivariatePolynomial);
    }
    Ok(UnivariatePolynomial::new(
        (0..=atom.polynomial.degree(0))
            .map(|degree| {
                atom.polynomial
                    .coefficient_in(0, degree)
                    .coefficient(&Monomial::one())
            })
            .collect(),
    ))
}
