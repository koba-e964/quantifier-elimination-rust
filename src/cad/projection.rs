use crate::formula::Formula;
use crate::polynomial::{Polynomial, Variable};
use std::collections::BTreeSet;

/// Construct a Collins-style projection set for one variable.
///
/// The result contains all nonzero coefficients, pairwise resultants, and
/// discriminant resultants of the supplied polynomials after projecting
/// `variable` away.
pub fn project(polynomials: &[Polynomial], variable: Variable) -> Vec<Polynomial> {
    project_with_limit(polynomials, variable, None, 0, &[]).expect("unbounded projection")
}

fn project_with_limit(
    polynomials: &[Polynomial],
    variable: Variable,
    max_polynomials: Option<usize>,
    projection_level: usize,
    variable_order: &[Variable],
) -> Result<Vec<Polynomial>, ProjectionError> {
    let mut result = Vec::new();
    for polynomial in polynomials {
        for degree in 0..=polynomial.degree(variable) {
            push_unique_with_limit(
                &mut result,
                polynomial.coefficient_in(variable, degree),
                max_polynomials,
                projection_level,
                variable_order,
            )?;
        }
        if polynomial.degree(variable) > 0 {
            let derivative = polynomial.derivative(variable);
            push_unique_with_limit(
                &mut result,
                resultant(polynomial, &derivative, variable),
                max_polynomials,
                projection_level,
                variable_order,
            )?;
        }
    }

    for left in 0..polynomials.len() {
        for right in (left + 1)..polynomials.len() {
            push_unique_with_limit(
                &mut result,
                resultant(&polynomials[left], &polynomials[right], variable),
                max_polynomials,
                projection_level,
                variable_order,
            )?;
        }
    }
    Ok(result)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionStack {
    pub variable_order: Vec<Variable>,
    pub levels: Vec<Vec<Polynomial>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    DuplicateVariable(Variable),
    MissingVariable(Variable),
    ComplexityLimitExceeded {
        limit: usize,
        polynomials: usize,
        projection_level: usize,
        variable_order: Vec<Variable>,
    },
}

/// Build projection sets from the input atoms, eliminating variables from
/// highest to lowest according to `variable_order`.
pub fn build_projection_stack(
    formula: &Formula,
    variable_order: &[Variable],
) -> Result<ProjectionStack, ProjectionError> {
    build_projection_stack_with_limit(formula, variable_order, None)
}

pub fn build_projection_stack_with_limit(
    formula: &Formula,
    variable_order: &[Variable],
    max_polynomials: Option<usize>,
) -> Result<ProjectionStack, ProjectionError> {
    let polynomials = formula_polynomials(formula);
    let required = polynomials
        .iter()
        .flat_map(Polynomial::variables)
        .collect::<BTreeSet<_>>();
    let provided = variable_order.iter().copied().collect::<BTreeSet<_>>();
    if provided.len() != variable_order.len() {
        let duplicate = variable_order
            .iter()
            .find(|variable| {
                variable_order
                    .iter()
                    .filter(|other| other == variable)
                    .count()
                    > 1
            })
            .copied()
            .unwrap();
        return Err(ProjectionError::DuplicateVariable(duplicate));
    }
    if let Some(missing) = required.difference(&provided).next() {
        return Err(ProjectionError::MissingVariable(*missing));
    }
    let mut current = polynomials;
    if let Some(limit) = max_polynomials {
        if current.len() > limit {
            return Err(ProjectionError::ComplexityLimitExceeded {
                limit,
                polynomials: current.len(),
                projection_level: 0,
                variable_order: variable_order.to_vec(),
            });
        }
    }
    let mut levels = vec![current.clone()];
    for (projection_level, variable) in variable_order.iter().rev().copied().enumerate() {
        current = project_with_limit(
            &current,
            variable,
            max_polynomials,
            projection_level + 1,
            variable_order,
        )?;
        if let Some(limit) = max_polynomials {
            if current.len() > limit {
                return Err(ProjectionError::ComplexityLimitExceeded {
                    limit,
                    polynomials: current.len(),
                    projection_level: projection_level + 1,
                    variable_order: variable_order.to_vec(),
                });
            }
        }
        levels.push(current.clone());
    }
    Ok(ProjectionStack {
        variable_order: variable_order.to_vec(),
        levels,
    })
}

pub(crate) fn formula_polynomials(formula: &Formula) -> Vec<Polynomial> {
    let mut polynomials = Vec::new();
    collect_formula_polynomials(formula, &mut polynomials);
    polynomials
}

fn collect_formula_polynomials(formula: &Formula, polynomials: &mut Vec<Polynomial>) {
    match formula {
        Formula::True | Formula::False => {}
        Formula::Atom(atom) => {
            if !polynomials.contains(&atom.polynomial) {
                polynomials.push(atom.polynomial.clone());
            }
        }
        Formula::Not(body) => collect_formula_polynomials(body, polynomials),
        Formula::And(formulas) | Formula::Or(formulas) => {
            for formula in formulas {
                collect_formula_polynomials(formula, polynomials);
            }
        }
        Formula::Quantified { body, .. } => collect_formula_polynomials(body, polynomials),
    }
}

pub fn resultant(left: &Polynomial, right: &Polynomial, variable: Variable) -> Polynomial {
    let left_degree = left.degree(variable);
    let right_degree = right.degree(variable);
    if left_degree == 0 && right_degree == 0 {
        return Polynomial::one();
    }
    if left_degree == 0 {
        return left.coefficient_in(variable, 0).pow(right_degree);
    }
    if right_degree == 0 {
        return right.coefficient_in(variable, 0).pow(left_degree);
    }

    let left_coefficients = (0..=left_degree)
        .map(|degree| left.coefficient_in(variable, degree))
        .collect::<Vec<_>>();
    let right_coefficients = (0..=right_degree)
        .map(|degree| right.coefficient_in(variable, degree))
        .collect::<Vec<_>>();
    let size = left_degree + right_degree;
    let mut matrix = vec![vec![Polynomial::zero(); size]; size];

    for row in 0..right_degree {
        for (column, coefficient) in left_coefficients.iter().enumerate() {
            matrix[row][row + column] = coefficient.clone();
        }
    }
    for row in 0..left_degree {
        for (column, coefficient) in right_coefficients.iter().enumerate() {
            matrix[right_degree + row][row + column] = coefficient.clone();
        }
    }
    determinant(&matrix)
}

fn determinant(matrix: &[Vec<Polynomial>]) -> Polynomial {
    if matrix.len() == 1 {
        return matrix[0][0].clone();
    }
    let mut result = Polynomial::zero();
    for column in 0..matrix.len() {
        let mut minor = Vec::with_capacity(matrix.len() - 1);
        for row in matrix.iter().skip(1) {
            let mut minor_row = Vec::with_capacity(matrix.len() - 1);
            for (index, value) in row.iter().enumerate() {
                if index != column {
                    minor_row.push(value.clone());
                }
            }
            minor.push(minor_row);
        }
        let term = matrix[0][column].clone() * determinant(&minor);
        result = if column % 2 == 0 {
            result + term
        } else {
            result - term
        };
    }
    result
}

fn push_unique(result: &mut Vec<Polynomial>, polynomial: Polynomial) {
    if !polynomial.is_zero() && !result.contains(&polynomial) {
        result.push(polynomial);
    }
}

fn push_unique_with_limit(
    result: &mut Vec<Polynomial>,
    polynomial: Polynomial,
    max_polynomials: Option<usize>,
    projection_level: usize,
    variable_order: &[Variable],
) -> Result<(), ProjectionError> {
    push_unique(result, polynomial);
    if let Some(limit) = max_polynomials {
        if result.len() > limit {
            return Err(ProjectionError::ComplexityLimitExceeded {
                limit,
                polynomials: result.len(),
                projection_level,
                variable_order: variable_order.to_vec(),
            });
        }
    }
    Ok(())
}
