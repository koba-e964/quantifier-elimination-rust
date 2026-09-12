use crate::algebra::univariate::UnivariatePolynomial;
use crate::cad::lifting::lift_two_variables;
use crate::cad::lifting::{
    decompose_univariate, synthesize_cell_conditions, FormulaEvaluationError,
};
use crate::cad::projection::ProjectionError;
use crate::formula::{Atom, Formula, Quantifier};
use crate::polynomial::Monomial;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantifierEvaluationError {
    Formula(FormulaEvaluationError),
    Projection(ProjectionError),
    NestedQuantifier,
    NonUnivariatePolynomial,
    WrongVariable,
}

impl From<ProjectionError> for QuantifierEvaluationError {
    fn from(error: ProjectionError) -> Self {
        Self::Projection(error)
    }
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

/// Eliminate the sole quantifier from a closed one-variable formula.
pub fn eliminate_univariate(formula: &Formula) -> Result<Formula, QuantifierEvaluationError> {
    Ok(if decide_univariate(formula)? {
        Formula::True
    } else {
        Formula::False
    })
}

/// Dispatch to the currently supported quantifier-elimination paths.
pub fn eliminate(formula: &Formula) -> Result<Formula, QuantifierEvaluationError> {
    let Formula::Quantified { variable, .. } = formula else {
        return Err(QuantifierEvaluationError::WrongVariable);
    };
    let free_variables = formula.free_variables();
    if free_variables.is_empty() {
        eliminate_univariate(formula)
    } else if free_variables.len() == 1 {
        eliminate_one_variable(formula, *free_variables.first().unwrap(), *variable)
    } else {
        Err(QuantifierEvaluationError::WrongVariable)
    }
}

/// Eliminate one quantified variable from a formula with exactly one free
/// variable. The current implementation uses the two-dimensional CAD layer.
pub fn eliminate_one_variable(
    formula: &Formula,
    free_variable: usize,
    quantified_variable: usize,
) -> Result<Formula, QuantifierEvaluationError> {
    let Formula::Quantified {
        quantifier,
        variable,
        body,
    } = formula
    else {
        return Err(QuantifierEvaluationError::WrongVariable);
    };
    let mut body_free_variables = body.free_variables();
    body_free_variables.remove(variable);
    if *variable != quantified_variable
        || body_free_variables != [free_variable].into_iter().collect()
    {
        return Err(QuantifierEvaluationError::WrongVariable);
    }
    let lifting = lift_two_variables(formula, &[free_variable, quantified_variable])?;
    let truth_table = lifting.truth_table(body)?;
    let base_truth = truth_table
        .iter()
        .map(|row| match quantifier {
            Quantifier::Exists => row.iter().any(|value| *value),
            Quantifier::Forall => row.iter().all(|value| *value),
        })
        .collect::<Vec<_>>();
    Ok(synthesize_cell_conditions(
        &lifting.base_cells,
        &lifting.base_polynomials,
        &base_truth,
        free_variable,
    ))
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
