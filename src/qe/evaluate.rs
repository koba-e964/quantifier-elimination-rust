use crate::algebra::univariate::UnivariatePolynomial;
use crate::cad::lifting::lift_two_variables;
use crate::cad::lifting::{
    decompose_univariate, synthesize_cell_conditions, FormulaEvaluationError, LiftingError,
};
use crate::cad::projection::ProjectionError;
use crate::formula::{Atom, Formula, Quantifier};
use crate::polynomial::Monomial;
use crate::qe::simplify::simplify;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantifierEvaluationError {
    Formula(FormulaEvaluationError),
    Projection(ProjectionError),
    Lifting(LiftingError),
    NestedQuantifier,
    NonUnivariatePolynomial,
    WrongVariable,
}

impl From<ProjectionError> for QuantifierEvaluationError {
    fn from(error: ProjectionError) -> Self {
        Self::Projection(error)
    }
}

impl From<LiftingError> for QuantifierEvaluationError {
    fn from(error: LiftingError) -> Self {
        Self::Lifting(error)
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
///
/// Currently supported inputs are closed one-variable formulas and formulas
/// with one quantified variable plus one free variable. General multivariate
/// formula synthesis is not enabled yet; formulas retaining two or more free
/// variables are rejected until a higher-dimensional lifting layer exists.
pub fn eliminate(formula: &Formula) -> Result<Formula, QuantifierEvaluationError> {
    if !matches!(formula, Formula::Quantified { .. }) {
        return Err(QuantifierEvaluationError::WrongVariable);
    }
    eliminate_recursive(formula)
}

fn eliminate_recursive(formula: &Formula) -> Result<Formula, QuantifierEvaluationError> {
    let Formula::Quantified {
        quantifier,
        variable,
        body,
    } = formula
    else {
        return Err(QuantifierEvaluationError::WrongVariable);
    };

    let body = simplify(&eliminate_nested_children(body)?);
    if !body.free_variables().contains(variable) {
        return Ok(simplify(&body));
    }
    let reduced = Formula::Quantified {
        quantifier: *quantifier,
        variable: *variable,
        body: Box::new(body),
    };
    if let Some(eliminated) = eliminate_linear_atom(&reduced, *variable) {
        return Ok(simplify(&eliminated));
    }
    let free_variables = formula.free_variables();
    if free_variables.is_empty() {
        eliminate_univariate(&reduced)
    } else if free_variables.len() == 1 {
        eliminate_one_variable(&reduced, *free_variables.first().unwrap(), *variable)
    } else {
        Err(QuantifierEvaluationError::WrongVariable)
    }
}

fn eliminate_linear_atom(formula: &Formula, variable: usize) -> Option<Formula> {
    let Formula::Quantified {
        quantifier, body, ..
    } = formula
    else {
        return None;
    };
    let Formula::Atom(atom) = body.as_ref() else {
        return None;
    };
    let polynomial = &atom.polynomial;
    if polynomial.degree(variable) != 1 {
        return None;
    }
    let leading = polynomial.coefficient_in(variable, 1);
    let constant = polynomial.coefficient_in(variable, 0);
    let leading_zero = Formula::atom(leading.clone(), crate::formula::Relation::Equal);
    let leading_nonzero = Formula::atom(leading, crate::formula::Relation::NotEqual);
    let constant_relation = |relation| Formula::atom(constant.clone(), relation);
    let result = match (quantifier, atom.relation) {
        (Quantifier::Exists, crate::formula::Relation::Equal) => Formula::Or(vec![
            Formula::And(vec![
                leading_zero,
                constant_relation(crate::formula::Relation::Equal),
            ]),
            leading_nonzero,
        ]),
        (Quantifier::Exists, crate::formula::Relation::NotEqual) => Formula::Or(vec![
            leading_nonzero,
            constant_relation(crate::formula::Relation::NotEqual),
        ]),
        (Quantifier::Exists, crate::formula::Relation::Less) => Formula::Or(vec![
            leading_nonzero,
            constant_relation(crate::formula::Relation::Less),
        ]),
        (Quantifier::Exists, crate::formula::Relation::LessOrEqual) => Formula::Or(vec![
            leading_nonzero,
            constant_relation(crate::formula::Relation::LessOrEqual),
        ]),
        (Quantifier::Exists, crate::formula::Relation::Greater) => Formula::Or(vec![
            leading_nonzero,
            constant_relation(crate::formula::Relation::Greater),
        ]),
        (Quantifier::Exists, crate::formula::Relation::GreaterOrEqual) => Formula::Or(vec![
            leading_nonzero,
            constant_relation(crate::formula::Relation::GreaterOrEqual),
        ]),
        (Quantifier::Forall, crate::formula::Relation::Equal) => Formula::And(vec![
            leading_zero,
            constant_relation(crate::formula::Relation::Equal),
        ]),
        (Quantifier::Forall, crate::formula::Relation::NotEqual) => Formula::And(vec![
            leading_zero,
            constant_relation(crate::formula::Relation::NotEqual),
        ]),
        (Quantifier::Forall, crate::formula::Relation::Less) => Formula::And(vec![
            leading_zero,
            constant_relation(crate::formula::Relation::Less),
        ]),
        (Quantifier::Forall, crate::formula::Relation::LessOrEqual) => Formula::And(vec![
            leading_zero,
            constant_relation(crate::formula::Relation::LessOrEqual),
        ]),
        (Quantifier::Forall, crate::formula::Relation::Greater) => Formula::And(vec![
            leading_zero,
            constant_relation(crate::formula::Relation::Greater),
        ]),
        (Quantifier::Forall, crate::formula::Relation::GreaterOrEqual) => Formula::And(vec![
            leading_zero,
            constant_relation(crate::formula::Relation::GreaterOrEqual),
        ]),
    };
    Some(result)
}

fn eliminate_nested_children(formula: &Formula) -> Result<Formula, QuantifierEvaluationError> {
    Ok(match formula {
        Formula::True | Formula::False | Formula::Atom(_) => formula.clone(),
        Formula::Not(body) => Formula::Not(Box::new(eliminate_nested_children(body)?)),
        Formula::And(formulas) => Formula::And(
            formulas
                .iter()
                .map(eliminate_nested_children)
                .collect::<Result<_, _>>()?,
        ),
        Formula::Or(formulas) => Formula::Or(
            formulas
                .iter()
                .map(eliminate_nested_children)
                .collect::<Result<_, _>>()?,
        ),
        Formula::Quantified { .. } => eliminate_recursive(formula)?,
    })
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
