use crate::algebra::algebraic::AlgebraicReal;
use crate::algebra::univariate::{RootInterval, UnivariatePolynomial};
use crate::cad::projection::{
    build_projection_stack, formula_polynomials, ProjectionError, ProjectionStack,
};
use crate::formula::Formula;
use crate::formula::Relation;
use crate::polynomial::{Polynomial, PolynomialEvaluationError, Variable};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellKind {
    Sector,
    Section,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnivariateCell {
    pub kind: CellKind,
    pub sample: BigRational,
    pub root: Option<RootInterval>,
    pub exact_sample: Option<AlgebraicReal>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormulaEvaluationError {
    NonUnivariatePolynomial,
    QuantifierNotSupported,
    PolynomialEvaluation(PolynomialEvaluationError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiftingError {
    Projection(ProjectionError),
    AlgebraicBaseSampleUnsupported,
}

impl From<ProjectionError> for LiftingError {
    fn from(error: ProjectionError) -> Self {
        Self::Projection(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoDimensionalLifting {
    pub projection_stack: ProjectionStack,
    pub base_polynomials: Vec<UnivariatePolynomial>,
    pub base_cells: Vec<UnivariateCell>,
    pub base_signs: Vec<Vec<i8>>,
    pub lifted_cells: Vec<Vec<UnivariateCell>>,
    pub lifted_signs: Vec<Vec<Vec<i8>>>,
}

impl TwoDimensionalLifting {
    pub fn truth_table(&self, formula: &Formula) -> Result<Vec<Vec<bool>>, FormulaEvaluationError> {
        let variable_order = &self.projection_stack.variable_order;
        self.base_cells
            .iter()
            .zip(&self.lifted_cells)
            .map(|(base, lifted)| {
                let mut values = std::collections::BTreeMap::new();
                values.insert(variable_order[0], base.sample.clone());
                lifted
                    .iter()
                    .map(|cell| {
                        evaluate_formula_at_lifted_cell(formula, variable_order[1], &values, cell)
                    })
                    .collect()
            })
            .collect()
    }
}

impl UnivariateCell {
    fn sector(sample: BigRational) -> Self {
        Self {
            kind: CellKind::Sector,
            sample,
            root: None,
            exact_sample: None,
        }
    }

    fn section(polynomial: UnivariatePolynomial, root: RootInterval) -> Self {
        Self {
            sample: (&root.lower + &root.upper) / BigInt::from(2),
            kind: CellKind::Section,
            exact_sample: Some(AlgebraicReal::new(polynomial, root.clone())),
            root: Some(root),
        }
    }

    pub fn sign_of(&self, polynomial: &UnivariatePolynomial) -> i8 {
        if let Some(algebraic) = &self.exact_sample {
            algebraic.sign_of(polynomial)
        } else {
            let value = polynomial.evaluate(&self.sample);
            if value.is_zero() {
                0
            } else if value.is_positive() {
                1
            } else {
                -1
            }
        }
    }

    pub fn satisfies(&self, polynomial: &UnivariatePolynomial, relation: Relation) -> bool {
        match relation {
            Relation::Equal => self.sign_of(polynomial) == 0,
            Relation::NotEqual => self.sign_of(polynomial) != 0,
            Relation::Less => self.sign_of(polynomial) < 0,
            Relation::LessOrEqual => self.sign_of(polynomial) <= 0,
            Relation::Greater => self.sign_of(polynomial) > 0,
            Relation::GreaterOrEqual => self.sign_of(polynomial) >= 0,
        }
    }

    pub fn evaluate_formula(&self, formula: &Formula) -> Result<bool, FormulaEvaluationError> {
        match formula {
            Formula::True => Ok(true),
            Formula::False => Ok(false),
            Formula::Atom(atom) => {
                let polynomial = to_univariate(&atom.polynomial)?;
                Ok(self.satisfies(&polynomial, atom.relation))
            }
            Formula::Not(body) => Ok(!self.evaluate_formula(body)?),
            Formula::And(formulas) => formulas
                .iter()
                .map(|formula| self.evaluate_formula(formula))
                .try_fold(true, |result, value| Ok(result && value?)),
            Formula::Or(formulas) => formulas
                .iter()
                .map(|formula| self.evaluate_formula(formula))
                .try_fold(false, |result, value| Ok(result || value?)),
            Formula::Quantified { .. } => Err(FormulaEvaluationError::QuantifierNotSupported),
        }
    }
}

/// Build the one-dimensional CAD induced by a family of univariate
/// polynomials. Sections contain one real root; sectors contain no roots.
pub fn decompose_univariate(polynomials: &[UnivariatePolynomial]) -> Vec<UnivariateCell> {
    let mut roots = polynomials
        .iter()
        .flat_map(|polynomial| {
            polynomial
                .isolate_real_roots()
                .into_iter()
                .map(|root| (polynomial.clone(), root))
        })
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left.1.lower.cmp(&right.1.lower));
    separate_and_deduplicate_roots(&mut roots);

    if roots.is_empty() {
        return vec![UnivariateCell::sector(BigRational::zero())];
    }

    let mut cells = Vec::with_capacity(roots.len() * 2 + 1);
    cells.push(UnivariateCell::sector(
        &roots[0].1.lower - BigRational::from_integer(BigInt::from(1)),
    ));
    for (index, (polynomial, root)) in roots.iter().cloned().enumerate() {
        cells.push(UnivariateCell::section(polynomial, root.clone()));
        if let Some(next) = roots.get(index + 1) {
            cells.push(UnivariateCell::sector(
                (&root.upper + &next.1.lower) / BigInt::from(2),
            ));
        } else {
            cells.push(UnivariateCell::sector(
                &root.upper + BigRational::from_integer(BigInt::from(1)),
            ));
        }
    }
    cells
}

fn separate_and_deduplicate_roots(roots: &mut Vec<(UnivariatePolynomial, RootInterval)>) {
    let mut index = 0;
    while index + 1 < roots.len() {
        let overlaps = roots[index].1.upper > roots[index + 1].1.lower
            && roots[index + 1].1.upper > roots[index].1.lower;
        if !overlaps {
            index += 1;
            continue;
        }

        let intersection = RootInterval::new(
            roots[index]
                .1
                .lower
                .clone()
                .max(roots[index + 1].1.lower.clone()),
            roots[index]
                .1
                .upper
                .clone()
                .min(roots[index + 1].1.upper.clone()),
        );
        let common = roots[index].0.gcd(&roots[index + 1].0);
        if common.count_roots(&intersection) > 0 {
            roots.remove(index + 1);
            continue;
        }

        let left_width = roots[index].1.width() / BigInt::from(2);
        let right_width = roots[index + 1].1.width() / BigInt::from(2);
        roots[index].1 = roots[index].0.refine_root(&roots[index].1, &left_width);
        roots[index + 1].1 = roots[index + 1]
            .0
            .refine_root(&roots[index + 1].1, &right_width);
        roots.sort_by(|left, right| left.1.lower.cmp(&right.1.lower));
    }
}

/// Specialize all variables other than `variable` at a rational sample point
/// and lift the resulting univariate family into cells.
pub fn lift_over_rational_sample(
    polynomials: &[Polynomial],
    variable: Variable,
    values: &std::collections::BTreeMap<Variable, BigRational>,
) -> Vec<UnivariateCell> {
    let specialized = polynomials
        .iter()
        .map(|polynomial| specialize_to_univariate(polynomial, variable, values))
        .collect::<Vec<_>>();
    decompose_univariate(&specialized)
}

/// Build the first recursive lifting layer for a two-variable formula. Lower
/// cells are sampled rationally until exact algebraic substitution is wired
/// into the multivariate evaluator.
pub fn lift_two_variables(
    formula: &Formula,
    variable_order: &[Variable; 2],
) -> Result<TwoDimensionalLifting, LiftingError> {
    let stack = build_projection_stack(formula, variable_order)?;
    let base_polynomials = stack.levels[1]
        .iter()
        .filter(|polynomial| {
            polynomial
                .variables()
                .all(|variable| variable == variable_order[0])
        })
        .map(|polynomial| {
            specialize_to_univariate(
                polynomial,
                variable_order[0],
                &std::collections::BTreeMap::new(),
            )
        })
        .collect::<Vec<_>>();
    let base_cells = decompose_univariate(&base_polynomials);
    let base_signs = base_cells
        .iter()
        .map(|cell| {
            base_polynomials
                .iter()
                .map(|polynomial| cell.sign_of(polynomial))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let original = formula_polynomials(formula);
    let mut lifted_cells = Vec::new();
    let mut lifted_signs = Vec::new();
    for cell in &base_cells {
        if let Some(exact_sample) = &cell.exact_sample {
            if exact_sample.rational_value().is_none() {
                return Err(LiftingError::AlgebraicBaseSampleUnsupported);
            }
        }
        let mut values = std::collections::BTreeMap::new();
        values.insert(variable_order[0], cell.sample.clone());
        let specialized = original
            .iter()
            .map(|polynomial| specialize_to_univariate(polynomial, variable_order[1], &values))
            .collect::<Vec<_>>();
        let cells = decompose_univariate(&specialized);
        let signs = cells
            .iter()
            .map(|lifted| {
                specialized
                    .iter()
                    .map(|polynomial| lifted.sign_of(polynomial))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        lifted_cells.push(cells);
        lifted_signs.push(signs);
    }
    Ok(TwoDimensionalLifting {
        projection_stack: stack,
        base_polynomials,
        base_cells,
        base_signs,
        lifted_cells,
        lifted_signs,
    })
}

pub fn evaluate_formula_at_lifted_cell(
    formula: &Formula,
    variable: Variable,
    values: &std::collections::BTreeMap<Variable, BigRational>,
    cell: &UnivariateCell,
) -> Result<bool, FormulaEvaluationError> {
    match formula {
        Formula::True => Ok(true),
        Formula::False => Ok(false),
        Formula::Atom(atom) => {
            let polynomial = specialize_to_univariate(&atom.polynomial, variable, values);
            Ok(cell.satisfies(&polynomial, atom.relation))
        }
        Formula::Not(body) => Ok(!evaluate_formula_at_lifted_cell(
            body, variable, values, cell,
        )?),
        Formula::And(formulas) => formulas.iter().try_fold(true, |result, formula| {
            Ok(result && evaluate_formula_at_lifted_cell(formula, variable, values, cell)?)
        }),
        Formula::Or(formulas) => formulas.iter().try_fold(false, |result, formula| {
            Ok(result || evaluate_formula_at_lifted_cell(formula, variable, values, cell)?)
        }),
        Formula::Quantified { .. } => Err(FormulaEvaluationError::QuantifierNotSupported),
    }
}

pub fn evaluate_formula_at_exact_lifted_cell(
    formula: &Formula,
    base_variable: Variable,
    lifted_variable: Variable,
    base_cell: &UnivariateCell,
    lifted_cell: &UnivariateCell,
) -> Result<bool, FormulaEvaluationError> {
    let mut values = std::collections::BTreeMap::new();
    values.insert(base_variable, exact_value(base_cell));
    values.insert(lifted_variable, exact_value(lifted_cell));
    evaluate_formula_at_exact_values(formula, &values)
}

fn evaluate_formula_at_exact_values(
    formula: &Formula,
    values: &std::collections::BTreeMap<Variable, crate::algebra::coefficient::ExactReal>,
) -> Result<bool, FormulaEvaluationError> {
    match formula {
        Formula::True => Ok(true),
        Formula::False => Ok(false),
        Formula::Atom(atom) => {
            let value = atom
                .polynomial
                .evaluate_exact(values)
                .map_err(FormulaEvaluationError::PolynomialEvaluation)?;
            let sign = value.sign();
            Ok(match atom.relation {
                Relation::Equal => sign == std::cmp::Ordering::Equal,
                Relation::NotEqual => sign != std::cmp::Ordering::Equal,
                Relation::Less => sign == std::cmp::Ordering::Less,
                Relation::LessOrEqual => sign != std::cmp::Ordering::Greater,
                Relation::Greater => sign == std::cmp::Ordering::Greater,
                Relation::GreaterOrEqual => sign != std::cmp::Ordering::Less,
            })
        }
        Formula::Not(body) => Ok(!evaluate_formula_at_exact_values(body, values)?),
        Formula::And(formulas) => formulas.iter().try_fold(true, |result, formula| {
            Ok(result && evaluate_formula_at_exact_values(formula, values)?)
        }),
        Formula::Or(formulas) => formulas.iter().try_fold(false, |result, formula| {
            Ok(result || evaluate_formula_at_exact_values(formula, values)?)
        }),
        Formula::Quantified { .. } => Err(FormulaEvaluationError::QuantifierNotSupported),
    }
}

fn exact_value(cell: &UnivariateCell) -> crate::algebra::coefficient::ExactReal {
    cell.exact_sample
        .clone()
        .map(crate::algebra::coefficient::ExactReal::algebraic)
        .unwrap_or_else(|| crate::algebra::coefficient::ExactReal::rational(cell.sample.clone()))
}

pub fn cell_condition(
    cell: &UnivariateCell,
    polynomials: &[UnivariatePolynomial],
    variable: Variable,
) -> Formula {
    let atoms = polynomials
        .iter()
        .map(|polynomial| {
            let relation = match cell.sign_of(polynomial) {
                value if value < 0 => Relation::Less,
                0 => Relation::Equal,
                _ => Relation::Greater,
            };
            Formula::atom(Polynomial::from_univariate(variable, polynomial), relation)
        })
        .collect();
    Formula::And(atoms)
}

pub fn synthesize_cell_conditions(
    cells: &[UnivariateCell],
    polynomials: &[UnivariatePolynomial],
    truth_values: &[bool],
    variable: Variable,
) -> Formula {
    assert_eq!(cells.len(), truth_values.len(), "one truth value per cell");
    if truth_values.iter().all(|truth| *truth) {
        return Formula::True;
    }
    if truth_values.iter().all(|truth| !*truth) {
        return Formula::False;
    }
    let conditions = cells
        .iter()
        .zip(truth_values)
        .filter(|(_, truth)| **truth)
        .map(|(cell, _)| cell_condition(cell, polynomials, variable))
        .collect::<Vec<_>>();
    Formula::Or(conditions)
}

fn specialize_to_univariate(
    polynomial: &Polynomial,
    variable: Variable,
    values: &std::collections::BTreeMap<Variable, BigRational>,
) -> UnivariatePolynomial {
    let mut coefficients = vec![BigRational::zero(); polynomial.degree(variable) + 1];
    for (monomial, coefficient) in polynomial.terms() {
        let mut value = coefficient.clone();
        for other_variable in monomial.variables() {
            if other_variable == variable {
                continue;
            }
            value *= values
                .get(&other_variable)
                .cloned()
                .unwrap_or_default()
                .pow(monomial.exponent(other_variable) as i32);
        }
        coefficients[monomial.exponent(variable)] += value;
    }
    UnivariatePolynomial::new(coefficients)
}

fn to_univariate(polynomial: &Polynomial) -> Result<UnivariatePolynomial, FormulaEvaluationError> {
    if polynomial.variables().any(|other| other != 0) {
        return Err(FormulaEvaluationError::NonUnivariatePolynomial);
    }
    Ok(UnivariatePolynomial::new(
        (0..=polynomial.degree(0))
            .map(|degree| {
                polynomial
                    .coefficient_in(0, degree)
                    .coefficient(&crate::polynomial::Monomial::one())
            })
            .collect(),
    ))
}
