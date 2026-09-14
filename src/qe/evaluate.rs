use crate::algebra::univariate::UnivariatePolynomial;
use crate::cad::lifting::{
    decompose_univariate, synthesize_cell_conditions, CadCell, CellKind, FormulaEvaluationError,
    LiftingError, RecursiveLifting, TwoDimensionalLifting, UnivariateCell,
};
use crate::cad::lifting::{lift_recursive, lift_two_variables};
use crate::cad::projection::ProjectionError;
use crate::formula::{Atom, Formula, Quantifier};
use crate::polynomial::Monomial;
use crate::qe::simplify::simplify;
use crate::qe::special::eliminate_symmetric_pair;
pub use crate::qe::special::{SpecialHandlingConfig, SpecialRule};
use num_traits::Signed;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantifierEvaluationError {
    Formula(FormulaEvaluationError),
    Projection(ProjectionError),
    Lifting(LiftingError),
    NestedQuantifier,
    NonUnivariatePolynomial,
    WrongVariable,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EliminationStats {
    pub quantifier_calls: usize,
    pub cells_constructed: usize,
    pub leaf_cells: usize,
    pub sector_cells: usize,
    pub section_cells: usize,
    pub projection_levels: usize,
    pub projection_polynomials: usize,
    pub lifting_orders: Vec<Vec<usize>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EliminationOptions {
    pub special_handling: bool,
    pub special_rules: SpecialHandlingConfig,
    pub variable_order: Option<Vec<usize>>,
}

impl Default for EliminationOptions {
    fn default() -> Self {
        Self {
            special_handling: true,
            special_rules: SpecialHandlingConfig::default(),
            variable_order: None,
        }
    }
}

impl EliminationStats {
    fn record_kind(&mut self, kind: &CellKind) {
        match kind {
            CellKind::Sector => self.sector_cells += 1,
            CellKind::Section => self.section_cells += 1,
        }
    }

    fn record_univariate_cells(&mut self, cells: &[UnivariateCell]) {
        self.cells_constructed += cells.len();
        self.leaf_cells += cells.len();
        for cell in cells {
            self.record_kind(&cell.kind);
        }
    }

    fn record_projection_stack(&mut self, stack: &crate::cad::projection::ProjectionStack) {
        self.projection_levels += stack.levels.len().saturating_sub(1);
        self.projection_polynomials += stack.levels.iter().map(Vec::len).sum::<usize>();
    }

    fn record_two_dimensional_lifting(&mut self, lifting: &TwoDimensionalLifting) {
        self.record_projection_stack(&lifting.projection_stack);
        self.record_univariate_cells(&lifting.base_cells);
        for cells in &lifting.lifted_cells {
            self.record_univariate_cells(cells);
        }
    }

    fn record_recursive_lifting(&mut self, lifting: &RecursiveLifting) {
        self.record_projection_stack(&lifting.projection_stack);
        self.lifting_orders
            .push(lifting.projection_stack.variable_order.clone());
        for cell in &lifting.cells {
            self.record_cad_cell(cell);
        }
    }

    fn record_cad_cell(&mut self, cell: &CadCell) {
        self.cells_constructed += 1;
        self.record_kind(&cell.coordinate.kind);
        if cell.children.is_empty() {
            self.leaf_cells += 1;
        } else {
            for child in &cell.children {
                self.record_cad_cell(child);
            }
        }
    }
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
    decide_univariate_with_stats(formula, &mut EliminationStats::default())
}

fn decide_univariate_with_stats(
    formula: &Formula,
    stats: &mut EliminationStats,
) -> Result<bool, QuantifierEvaluationError> {
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
    stats.record_univariate_cells(&cells);
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
/// Currently supported inputs include closed one-variable formulas, formulas
/// with one quantified variable plus free variables of arbitrary dimension,
/// nested quantifiers whose inner results can be synthesized, and selected
/// multivariate linear-atomic or distributable Boolean cases. General
/// algebraic-coefficient arithmetic beyond the exact operations implemented by
/// the lifting layer remains explicitly unsupported.
pub fn eliminate(formula: &Formula) -> Result<Formula, QuantifierEvaluationError> {
    eliminate_with_stats(formula).map(|(result, _)| result)
}

pub fn eliminate_with_stats(
    formula: &Formula,
) -> Result<(Formula, EliminationStats), QuantifierEvaluationError> {
    eliminate_with_options(formula, EliminationOptions::default())
}

pub fn eliminate_with_options(
    formula: &Formula,
    options: EliminationOptions,
) -> Result<(Formula, EliminationStats), QuantifierEvaluationError> {
    let mut stats = EliminationStats::default();
    if formula.is_quantifier_free() {
        return Ok((simplify(formula), stats));
    }
    if options.special_handling
        && options
            .special_rules
            .enabled_rules
            .contains(&SpecialRule::SymmetricSumProduct)
    {
        if let Some(result) = eliminate_symmetric_pair(formula) {
            return Ok((result, stats));
        }
    }
    let result = eliminate_recursive(formula, &mut stats, options)?;
    Ok((result, stats))
}

fn eliminate_recursive(
    formula: &Formula,
    stats: &mut EliminationStats,
    options: EliminationOptions,
) -> Result<Formula, QuantifierEvaluationError> {
    stats.quantifier_calls += 1;
    let Formula::Quantified {
        quantifier,
        variable,
        body,
    } = formula
    else {
        return Err(QuantifierEvaluationError::WrongVariable);
    };

    let body = simplify(&eliminate_nested_children(body, stats, options.clone())?);
    if !body.free_variables().contains(variable) {
        return Ok(simplify(&body));
    }
    let reduced = Formula::Quantified {
        quantifier: *quantifier,
        variable: *variable,
        body: Box::new(body.clone()),
    };
    if let Some(eliminated) = eliminate_forall_linear_equality(&reduced, *variable) {
        return Ok(eliminated);
    }
    if let Some(eliminated) = eliminate_forall_linear_conjunction(&reduced, *variable) {
        return Ok(eliminated);
    }
    if let Some(eliminated) = eliminate_exists_linear_conjunction(&reduced, *variable) {
        return Ok(simplify(&eliminated));
    }
    if let Some(eliminated) = eliminate_exists_linear_inequalities(&reduced, *variable) {
        return Ok(simplify(&eliminated));
    }
    if let Some(eliminated) = eliminate_supported_boolean_branches(
        *quantifier,
        *variable,
        &reduced,
        stats,
        options.clone(),
    ) {
        return Ok(simplify(&eliminated?));
    }
    if let Some(eliminated) = eliminate_linear_atom(&reduced, *variable) {
        return Ok(simplify(&eliminated));
    }
    let free_variables = formula.free_variables();
    if free_variables.is_empty() {
        Ok(if decide_univariate_with_stats(&reduced, stats)? {
            Formula::True
        } else {
            Formula::False
        })
    } else if free_variables.len() == 1 {
        eliminate_one_variable_with_stats(
            &reduced,
            *free_variables.first().unwrap(),
            *variable,
            stats,
        )
    } else {
        let mut variable_order = options
            .variable_order
            .clone()
            .filter(|order| {
                order.len() == free_variables.len()
                    && order
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                        == free_variables
            })
            .unwrap_or_else(|| select_variable_order(&reduced, &free_variables, *variable));
        variable_order.push(*variable);
        let lifting = lift_recursive(&reduced, &variable_order)?;
        stats.record_recursive_lifting(&lifting);
        lifting
            .synthesize_quantifier(&body, *quantifier, *variable)
            .map(|result| simplify(&result))
            .map_err(QuantifierEvaluationError::from)
    }
}

fn select_variable_order(
    formula: &Formula,
    free_variables: &std::collections::BTreeSet<usize>,
    quantified_variable: usize,
) -> Vec<usize> {
    let mut scores = std::collections::BTreeMap::<usize, (usize, usize)>::new();
    collect_variable_order_scores(formula, free_variables, quantified_variable, &mut scores);

    let mut variables = free_variables.iter().copied().collect::<Vec<_>>();
    variables.sort_by_key(|variable| {
        let (coupling, degree) = scores.get(variable).copied().unwrap_or_default();
        (
            std::cmp::Reverse(coupling),
            std::cmp::Reverse(degree),
            *variable,
        )
    });
    variables
}

fn collect_variable_order_scores(
    formula: &Formula,
    free_variables: &std::collections::BTreeSet<usize>,
    quantified_variable: usize,
    scores: &mut std::collections::BTreeMap<usize, (usize, usize)>,
) {
    match formula {
        Formula::True | Formula::False => {}
        Formula::Atom(atom) => {
            if atom
                .polynomial
                .variables()
                .any(|variable| variable == quantified_variable)
            {
                for variable in free_variables.iter().copied() {
                    if atom
                        .polynomial
                        .variables()
                        .any(|candidate| candidate == variable)
                    {
                        let entry = scores.entry(variable).or_default();
                        entry.0 += 1;
                        entry.1 += atom.polynomial.degree(variable);
                    }
                }
            }
        }
        Formula::Not(body) => {
            collect_variable_order_scores(body, free_variables, quantified_variable, scores)
        }
        Formula::And(formulas) | Formula::Or(formulas) => {
            for formula in formulas {
                collect_variable_order_scores(formula, free_variables, quantified_variable, scores);
            }
        }
        Formula::Quantified { body, .. } => {
            collect_variable_order_scores(body, free_variables, quantified_variable, scores)
        }
    }
}

fn eliminate_forall_linear_equality(formula: &Formula, variable: usize) -> Option<Formula> {
    let Formula::Quantified {
        quantifier: Quantifier::Forall,
        body,
        ..
    } = formula
    else {
        return None;
    };
    let Formula::And(branches) = body.as_ref() else {
        return None;
    };
    branches.iter().find_map(|branch| {
        let Formula::Atom(atom) = branch else {
            return None;
        };
        if atom.relation != crate::formula::Relation::Equal || atom.polynomial.degree(variable) != 1
        {
            return None;
        }
        let leading = atom.polynomial.coefficient_in(variable, 1);
        if leading.variables().next().is_some() || leading.is_zero() {
            return None;
        }
        Some(Formula::False)
    })
}

fn eliminate_forall_linear_conjunction(formula: &Formula, variable: usize) -> Option<Formula> {
    let Formula::Quantified {
        quantifier: Quantifier::Forall,
        body,
        ..
    } = formula
    else {
        return None;
    };
    let Formula::And(branches) = body.as_ref() else {
        return None;
    };
    branches.iter().find_map(|branch| {
        let atom = atom_with_negated_relation(branch)?;
        let coefficient = atom.polynomial.coefficient_in(variable, 1);
        if atom.polynomial.degree(variable) != 1
            || coefficient.variables().next().is_some()
            || coefficient.is_zero()
        {
            return None;
        }
        Some(Formula::False)
    })
}

fn eliminate_exists_linear_conjunction(formula: &Formula, variable: usize) -> Option<Formula> {
    let Formula::Quantified {
        quantifier: Quantifier::Exists,
        body,
        ..
    } = formula
    else {
        return None;
    };
    let Formula::And(branches) = body.as_ref() else {
        return None;
    };
    let equality = branches.iter().find_map(|branch| {
        let atom = atom_with_negated_relation(branch)?;
        if atom.relation != crate::formula::Relation::Equal || atom.polynomial.degree(variable) != 1
        {
            return None;
        }
        let leading = atom.polynomial.coefficient_in(variable, 1);
        if leading.variables().next().is_some() || leading.is_zero() {
            return None;
        }
        Some((leading, atom.polynomial.coefficient_in(variable, 0)))
    })?;
    let (leading, equality_constant) = equality;
    let mut conditions = Vec::new();
    for branch in branches {
        let atom = atom_with_negated_relation(branch)?;
        if atom.polynomial.degree(variable) == 0 {
            conditions.push(Formula::Atom(atom));
            continue;
        }
        if atom.polynomial.degree(variable) != 1 {
            return None;
        }
        let coefficient = atom.polynomial.coefficient_in(variable, 1);
        let constant = atom.polynomial.coefficient_in(variable, 0);
        let substituted = leading.clone() * constant - coefficient * equality_constant.clone();
        let relation = if leading.evaluate(&Default::default()).is_positive() {
            atom.relation
        } else {
            reverse_inequality(atom.relation)
        };
        conditions.push(Formula::atom(substituted, relation));
    }
    Some(Formula::And(conditions))
}

fn eliminate_exists_linear_inequalities(formula: &Formula, variable: usize) -> Option<Formula> {
    let Formula::Quantified {
        quantifier: Quantifier::Exists,
        body,
        ..
    } = formula
    else {
        return None;
    };
    let Formula::And(branches) = body.as_ref() else {
        return None;
    };
    let mut lower_bounds = Vec::new();
    let mut upper_bounds = Vec::new();
    let mut conditions = Vec::new();
    for branch in branches {
        let atom = atom_with_negated_relation(branch)?;
        match atom.polynomial.degree(variable) {
            0 => conditions.push(Formula::Atom(atom)),
            1 => {
                let coefficient = atom.polynomial.coefficient_in(variable, 1);
                if coefficient.variables().next().is_some() || coefficient.is_zero() {
                    return None;
                }
                let constant = atom.polynomial.coefficient_in(variable, 0);
                let positive = coefficient.evaluate(&Default::default()).is_positive();
                let (is_lower, strict) = match (positive, atom.relation) {
                    (true, crate::formula::Relation::Greater) => (true, true),
                    (true, crate::formula::Relation::GreaterOrEqual) => (true, false),
                    (true, crate::formula::Relation::Less) => (false, true),
                    (true, crate::formula::Relation::LessOrEqual) => (false, false),
                    (false, crate::formula::Relation::Greater) => (false, true),
                    (false, crate::formula::Relation::GreaterOrEqual) => (false, false),
                    (false, crate::formula::Relation::Less) => (true, true),
                    (false, crate::formula::Relation::LessOrEqual) => (true, false),
                    _ => return None,
                };
                if is_lower {
                    if positive {
                        lower_bounds.push((coefficient, constant, strict));
                    } else {
                        lower_bounds.push((-coefficient, -constant, strict));
                    }
                } else {
                    if positive {
                        upper_bounds.push((coefficient, constant, strict));
                    } else {
                        upper_bounds.push((-coefficient, -constant, strict));
                    }
                }
            }
            _ => return None,
        }
    }
    for (lower_coefficient, lower_constant, lower_strict) in &lower_bounds {
        for (upper_coefficient, upper_constant, upper_strict) in &upper_bounds {
            let difference = upper_coefficient.clone() * lower_constant.clone()
                - lower_coefficient.clone() * upper_constant.clone();
            let relation = if *lower_strict || *upper_strict {
                crate::formula::Relation::Greater
            } else {
                crate::formula::Relation::GreaterOrEqual
            };
            conditions.push(Formula::atom(difference, relation));
        }
    }
    Some(Formula::And(conditions))
}

fn atom_with_negated_relation(formula: &Formula) -> Option<Atom> {
    match formula {
        Formula::Atom(atom) => Some(atom.clone()),
        Formula::Not(body) => match body.as_ref() {
            Formula::Atom(atom) => Some(Atom::new(
                atom.polynomial.clone(),
                negate_relation(atom.relation),
            )),
            _ => None,
        },
        _ => None,
    }
}

fn reverse_inequality(relation: crate::formula::Relation) -> crate::formula::Relation {
    match relation {
        crate::formula::Relation::Less => crate::formula::Relation::Greater,
        crate::formula::Relation::LessOrEqual => crate::formula::Relation::GreaterOrEqual,
        crate::formula::Relation::Greater => crate::formula::Relation::Less,
        crate::formula::Relation::GreaterOrEqual => crate::formula::Relation::LessOrEqual,
        relation => relation,
    }
}

fn eliminate_supported_boolean_branches(
    quantifier: Quantifier,
    variable: usize,
    formula: &Formula,
    stats: &mut EliminationStats,
    options: EliminationOptions,
) -> Option<Result<Formula, QuantifierEvaluationError>> {
    let Formula::Quantified { body, .. } = formula else {
        return None;
    };
    match (quantifier, body.as_ref()) {
        (Quantifier::Exists, Formula::Or(branches)) => Some(
            branches
                .iter()
                .map(|branch| {
                    eliminate_recursive(
                        &Formula::exists(variable, branch.clone()),
                        stats,
                        options.clone(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Formula::Or),
        ),
        (Quantifier::Forall, Formula::And(branches)) => Some(
            branches
                .iter()
                .map(|branch| {
                    eliminate_recursive(
                        &Formula::forall(variable, branch.clone()),
                        stats,
                        options.clone(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Formula::And),
        ),
        (Quantifier::Exists, Formula::And(branches)) => {
            let (independent, dependent): (Vec<_>, Vec<_>) = branches
                .iter()
                .cloned()
                .partition(|branch| !branch.free_variables().contains(&variable));
            if independent.is_empty() || dependent.is_empty() {
                return None;
            }
            let guard = simplify(&Formula::And(independent));
            let quantified = Formula::exists(variable, simplify(&Formula::And(dependent)));
            Some(
                eliminate_recursive(&quantified, stats, options.clone())
                    .map(|result| Formula::And(vec![guard, result])),
            )
        }
        (Quantifier::Forall, Formula::Or(branches)) => {
            let (independent, dependent): (Vec<_>, Vec<_>) = branches
                .iter()
                .cloned()
                .partition(|branch| !branch.free_variables().contains(&variable));
            if independent.is_empty() {
                let negated = Formula::And(
                    branches
                        .iter()
                        .cloned()
                        .map(|branch| Formula::Not(Box::new(branch)))
                        .collect(),
                );
                let quantified = Formula::exists(variable, negated);
                return Some(
                    eliminate_recursive(&quantified, stats, options.clone())
                        .map(|result| Formula::Not(Box::new(result))),
                );
            }
            if dependent.is_empty() {
                return None;
            }
            let guard = simplify(&Formula::Or(independent));
            let quantified = Formula::forall(variable, simplify(&Formula::Or(dependent)));
            Some(
                eliminate_recursive(&quantified, stats, options.clone())
                    .map(|result| Formula::Or(vec![guard, result])),
            )
        }
        (Quantifier::Exists, Formula::Not(inner)) => Some(
            eliminate_recursive(
                &Formula::forall(variable, inner.as_ref().clone()),
                stats,
                options.clone(),
            )
            .map(|result| Formula::Not(Box::new(result))),
        ),
        (Quantifier::Forall, Formula::Not(inner)) => Some(
            eliminate_recursive(
                &Formula::exists(variable, inner.as_ref().clone()),
                stats,
                options.clone(),
            )
            .map(|result| Formula::Not(Box::new(result))),
        ),
        _ => None,
    }
}

fn eliminate_linear_atom(formula: &Formula, variable: usize) -> Option<Formula> {
    let Formula::Quantified {
        quantifier, body, ..
    } = formula
    else {
        return None;
    };
    let atom = match body.as_ref() {
        Formula::Atom(atom) => atom.clone(),
        Formula::Not(inner) => {
            let Formula::Atom(atom) = inner.as_ref() else {
                return None;
            };
            Atom::new(atom.polynomial.clone(), negate_relation(atom.relation))
        }
        _ => return None,
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

fn negate_relation(relation: crate::formula::Relation) -> crate::formula::Relation {
    match relation {
        crate::formula::Relation::Equal => crate::formula::Relation::NotEqual,
        crate::formula::Relation::NotEqual => crate::formula::Relation::Equal,
        crate::formula::Relation::Less => crate::formula::Relation::GreaterOrEqual,
        crate::formula::Relation::LessOrEqual => crate::formula::Relation::Greater,
        crate::formula::Relation::Greater => crate::formula::Relation::LessOrEqual,
        crate::formula::Relation::GreaterOrEqual => crate::formula::Relation::Less,
    }
}

fn eliminate_nested_children(
    formula: &Formula,
    stats: &mut EliminationStats,
    options: EliminationOptions,
) -> Result<Formula, QuantifierEvaluationError> {
    Ok(match formula {
        Formula::True | Formula::False | Formula::Atom(_) => formula.clone(),
        Formula::Not(body) => Formula::Not(Box::new(eliminate_nested_children(
            body,
            stats,
            options.clone(),
        )?)),
        Formula::And(formulas) => Formula::And(
            formulas
                .iter()
                .map(|formula| eliminate_nested_children(formula, stats, options.clone()))
                .collect::<Result<_, _>>()?,
        ),
        Formula::Or(formulas) => Formula::Or(
            formulas
                .iter()
                .map(|formula| eliminate_nested_children(formula, stats, options.clone()))
                .collect::<Result<_, _>>()?,
        ),
        Formula::Quantified { .. } => eliminate_recursive(formula, stats, options.clone())?,
    })
}

/// Eliminate one quantified variable from a formula with exactly one free
/// variable. The current implementation uses the two-dimensional CAD layer.
pub fn eliminate_one_variable(
    formula: &Formula,
    free_variable: usize,
    quantified_variable: usize,
) -> Result<Formula, QuantifierEvaluationError> {
    eliminate_one_variable_with_stats(
        formula,
        free_variable,
        quantified_variable,
        &mut EliminationStats::default(),
    )
}

fn eliminate_one_variable_with_stats(
    formula: &Formula,
    free_variable: usize,
    quantified_variable: usize,
    stats: &mut EliminationStats,
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
    stats.record_two_dimensional_lifting(&lifting);
    let truth_table = lifting.truth_table(body)?;
    let base_truth = truth_table
        .iter()
        .map(|row| match quantifier {
            Quantifier::Exists => row.iter().any(|value| *value),
            Quantifier::Forall => row.iter().all(|value| *value),
        })
        .collect::<Vec<_>>();
    Ok(simplify(&synthesize_cell_conditions(
        &lifting.base_cells,
        &lifting.base_polynomials,
        &base_truth,
        free_variable,
    )))
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
