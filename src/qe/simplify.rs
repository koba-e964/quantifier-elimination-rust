use crate::formula::{Formula, Relation};
use crate::polynomial::Polynomial;
use num_rational::BigRational;
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

fn simplify_conjunction(formulas: &[Formula]) -> Formula {
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

#[derive(Clone, Debug)]
enum LinearBound {
    Lower { value: BigRational, strict: bool },
    Upper { value: BigRational, strict: bool },
    Point(BigRational),
}

fn simplify_univariate_linear_bounds(formulas: &mut Vec<Formula>) -> bool {
    let mut variables = Vec::new();
    for formula in formulas.iter() {
        let Some(variable) = formula_linear_bound(formula).map(|(variable, _)| variable) else {
            continue;
        };
        if !variables.contains(&variable) {
            variables.push(variable);
        }
    }
    for variable in variables {
        let entries = formulas
            .iter()
            .enumerate()
            .filter_map(|(index, formula)| {
                formula_linear_bound(formula)
                    .filter(|(candidate, _)| *candidate == variable)
                    .map(|(_, bound)| (index, bound))
            })
            .collect::<Vec<_>>();
        if entries.len() < 2 {
            continue;
        }
        let mut point = None;
        let mut lower = None;
        let mut upper = None;
        for (index, bound) in &entries {
            match bound {
                LinearBound::Point(value) => {
                    if let Some((_, other)) = &point {
                        if other != value {
                            return true;
                        }
                    } else {
                        point = Some((*index, value.clone()));
                    }
                }
                LinearBound::Lower { value, strict } => {
                    if lower.as_ref().is_none_or(|(_, current, current_strict)| {
                        value > current || (value == current && *strict && !current_strict)
                    }) {
                        lower = Some((*index, value.clone(), *strict));
                    }
                }
                LinearBound::Upper { value, strict } => {
                    if upper.as_ref().is_none_or(|(_, current, current_strict)| {
                        value < current || (value == current && *strict && !current_strict)
                    }) {
                        upper = Some((*index, value.clone(), *strict));
                    }
                }
            }
        }
        if let Some((point_index, point_value)) = point {
            if lower.as_ref().is_some_and(|(_, value, strict)| {
                point_value < *value || (point_value == *value && *strict)
            }) || upper.as_ref().is_some_and(|(_, value, strict)| {
                point_value > *value || (point_value == *value && *strict)
            }) {
                return true;
            }
            let retained = entries
                .iter()
                .map(|(index, _)| *index)
                .filter(|index| *index != point_index)
                .collect::<Vec<_>>();
            for index in retained.into_iter().rev() {
                formulas.remove(index);
            }
            continue;
        }
        if let (Some((_, lower_value, lower_strict)), Some((_, upper_value, upper_strict))) =
            (&lower, &upper)
        {
            if lower_value > upper_value
                || (lower_value == upper_value && (*lower_strict || *upper_strict))
            {
                return true;
            }
        }
        let retained = lower
            .into_iter()
            .map(|(index, _, _)| index)
            .chain(upper.into_iter().map(|(index, _, _)| index))
            .collect::<Vec<_>>();
        for index in entries
            .iter()
            .map(|(index, _)| *index)
            .filter(|index| !retained.contains(index))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            formulas.remove(index);
        }
    }
    false
}

fn formula_linear_bound(formula: &Formula) -> Option<(usize, LinearBound)> {
    let Formula::Atom(atom) = formula else {
        return None;
    };
    let variables = atom.polynomial.variables().collect::<Vec<_>>();
    let [variable] = variables.as_slice() else {
        return None;
    };
    if atom.polynomial.degree(*variable) != 1 {
        return None;
    }
    let empty = BTreeMap::new();
    let coefficient = atom
        .polynomial
        .coefficient_in(*variable, 1)
        .evaluate(&empty);
    let constant = atom
        .polynomial
        .coefficient_in(*variable, 0)
        .evaluate(&empty);
    if coefficient.is_zero() {
        return None;
    }
    let root = -constant / coefficient.clone();
    let relation = if coefficient.is_negative() {
        negate_relation(atom.relation)
    } else {
        atom.relation
    };
    Some((
        *variable,
        match relation {
            Relation::Less => LinearBound::Upper {
                value: root,
                strict: true,
            },
            Relation::LessOrEqual => LinearBound::Upper {
                value: root,
                strict: false,
            },
            Relation::Greater => LinearBound::Lower {
                value: root,
                strict: true,
            },
            Relation::GreaterOrEqual => LinearBound::Lower {
                value: root,
                strict: false,
            },
            Relation::Equal => LinearBound::Point(root),
            Relation::NotEqual => return None,
        },
    ))
}

#[derive(Clone)]
struct LinearConstraint {
    coefficients: BTreeMap<usize, BigRational>,
    constant: BigRational,
    strict: bool,
}

fn simplify_redundant_linear_constraints(formulas: &mut Vec<Formula>) {
    loop {
        let mut removed = false;
        for index in 0..formulas.len() {
            let Formula::Atom(target) = &formulas[index] else {
                continue;
            };
            if target.relation == Relation::NotEqual || !is_linear_polynomial(&target.polynomial) {
                continue;
            }
            let others = formulas
                .iter()
                .enumerate()
                .filter(|(other_index, _)| *other_index != index)
                .map(|(_, formula)| formula.clone())
                .collect::<Vec<_>>();
            if others
                .iter()
                .any(|formula| !matches!(formula, Formula::Atom(atom) if atom.relation != Relation::NotEqual && is_linear_polynomial(&atom.polynomial)))
            {
                continue;
            }
            if linear_atom_implied(&others, target) {
                formulas.remove(index);
                removed = true;
                break;
            }
        }
        if !removed {
            return;
        }
    }
}

fn linear_conjunction_is_infeasible(formulas: &[Formula]) -> bool {
    let constraints = formulas
        .iter()
        .map(|formula| match formula {
            Formula::Atom(atom) => atom_constraints(atom, false),
            _ => None,
        })
        .collect::<Option<Vec<_>>>();
    let Some(constraints) = constraints else {
        return false;
    };
    !linear_constraints_feasible(constraints.into_iter().flatten().collect())
}

fn linear_atom_implied(others: &[Formula], target: &crate::formula::Atom) -> bool {
    let base = others
        .iter()
        .filter_map(|formula| {
            atom_constraints(
                match formula {
                    Formula::Atom(atom) => atom,
                    _ => unreachable!(),
                },
                false,
            )
        })
        .flatten()
        .collect::<Vec<_>>();
    if target.relation == Relation::Equal {
        [Relation::Greater, Relation::Less]
            .into_iter()
            .all(|relation| {
                let mut constraints = base.clone();
                constraints.extend(atom_constraints_for_relation(&target.polynomial, relation));
                !linear_constraints_feasible(constraints)
            })
    } else {
        let mut constraints = base;
        constraints.extend(
            atom_constraints(target, true_for_relation(target.relation))
                .into_iter()
                .flatten(),
        );
        !linear_constraints_feasible(constraints)
    }
}

fn true_for_relation(relation: Relation) -> bool {
    !matches!(relation, Relation::Equal)
}

fn atom_constraints(atom: &crate::formula::Atom, negate: bool) -> Option<Vec<LinearConstraint>> {
    if !is_linear_polynomial(&atom.polynomial) || atom.relation == Relation::NotEqual {
        return None;
    }
    let relation = if negate {
        negate_relation(atom.relation)
    } else {
        atom.relation
    };
    Some(atom_constraints_for_relation(&atom.polynomial, relation))
}

fn atom_constraints_for_relation(
    polynomial: &Polynomial,
    relation: Relation,
) -> Vec<LinearConstraint> {
    let (coefficients, constant) = linear_coefficients(polynomial).unwrap();
    let mut constraints = Vec::new();
    match relation {
        Relation::Greater => constraints.push(LinearConstraint {
            coefficients,
            constant,
            strict: true,
        }),
        Relation::GreaterOrEqual => constraints.push(LinearConstraint {
            coefficients,
            constant,
            strict: false,
        }),
        Relation::Less => constraints.push(negated_constraint(coefficients, constant, true)),
        Relation::LessOrEqual => {
            constraints.push(negated_constraint(coefficients, constant, false))
        }
        Relation::Equal => {
            constraints.push(LinearConstraint {
                coefficients: coefficients.clone(),
                constant: constant.clone(),
                strict: false,
            });
            constraints.push(negated_constraint(coefficients, constant, false));
        }
        Relation::NotEqual => return Vec::new(),
    }
    constraints
}

fn negated_constraint(
    coefficients: BTreeMap<usize, BigRational>,
    constant: BigRational,
    strict: bool,
) -> LinearConstraint {
    LinearConstraint {
        coefficients: coefficients
            .into_iter()
            .map(|(variable, coefficient)| (variable, -coefficient))
            .collect(),
        constant: -constant,
        strict,
    }
}

fn is_linear_polynomial(polynomial: &Polynomial) -> bool {
    polynomial
        .terms()
        .all(|(monomial, _)| monomial.total_degree() <= 1)
}

fn linear_coefficients(
    polynomial: &Polynomial,
) -> Option<(BTreeMap<usize, BigRational>, BigRational)> {
    if !is_linear_polynomial(polynomial) {
        return None;
    }
    let mut coefficients: BTreeMap<usize, BigRational> = BTreeMap::new();
    let mut constant = BigRational::zero();
    for (monomial, coefficient) in polynomial.terms() {
        if monomial.total_degree() == 0 {
            constant += coefficient.clone();
        } else {
            let variable = monomial.variables().next()?;
            *coefficients.entry(variable).or_default() += coefficient.clone();
        }
    }
    coefficients.retain(|_, coefficient| !coefficient.is_zero());
    Some((coefficients, constant))
}

fn linear_constraints_feasible(mut constraints: Vec<LinearConstraint>) -> bool {
    let variables = constraints
        .iter()
        .flat_map(|constraint| constraint.coefficients.keys().copied())
        .collect::<Vec<_>>();
    for variable in variables {
        let mut lower = Vec::new();
        let mut upper = Vec::new();
        let mut remaining = Vec::new();
        for mut constraint in constraints {
            let coefficient = constraint
                .coefficients
                .remove(&variable)
                .unwrap_or_default();
            if coefficient.is_zero() {
                remaining.push(constraint);
            } else if coefficient.is_positive() {
                lower.push((constraint, coefficient));
            } else {
                upper.push((constraint, coefficient));
            }
        }
        for (lower_constraint, lower_coefficient) in &lower {
            for (upper_constraint, upper_coefficient) in &upper {
                let mut coefficients: BTreeMap<usize, BigRational> = BTreeMap::new();
                for (other, coefficient) in &lower_constraint.coefficients {
                    *coefficients.entry(*other).or_default() += (-upper_coefficient) * coefficient;
                }
                for (other, coefficient) in &upper_constraint.coefficients {
                    *coefficients.entry(*other).or_default() += lower_coefficient * coefficient;
                }
                coefficients.retain(|_, coefficient| !coefficient.is_zero());
                remaining.push(LinearConstraint {
                    coefficients,
                    constant: (-upper_coefficient) * &lower_constraint.constant
                        + lower_coefficient * &upper_constraint.constant,
                    strict: lower_constraint.strict || upper_constraint.strict,
                });
            }
        }
        constraints = remaining;
    }
    constraints.into_iter().all(|constraint| {
        !constraint.coefficients.is_empty()
            || constraint.constant > BigRational::zero()
            || (constraint.constant == BigRational::zero() && !constraint.strict)
    })
}

fn merge_sign_constraints(formulas: &mut Vec<Formula>) -> bool {
    let mut index = 0;
    while index < formulas.len() {
        let Some((polynomial, relation)) = formulas.get(index).and_then(|formula| match formula {
            Formula::Atom(atom) => Some((atom.polynomial.clone(), atom.relation)),
            _ => None,
        }) else {
            index += 1;
            continue;
        };
        let Some(other_index) =
            formulas
                .iter()
                .enumerate()
                .skip(index + 1)
                .find_map(|(other_index, formula)| match formula {
                    Formula::Atom(atom) if atom.polynomial == polynomial => Some(other_index),
                    _ => None,
                })
        else {
            index += 1;
            continue;
        };
        let other_relation = match &formulas[other_index] {
            Formula::Atom(atom) => atom.relation,
            _ => unreachable!(),
        };
        let combined = relation_mask(relation) & relation_mask(other_relation);
        let Some(combined_relation) = relation_from_mask(combined) else {
            return true;
        };
        formulas.remove(other_index);
        formulas[index] = Formula::atom(polynomial, combined_relation);
        index = 0;
    }
    false
}

fn relation_mask(relation: Relation) -> u8 {
    match relation {
        Relation::Less => 1,
        Relation::Equal => 2,
        Relation::Greater => 4,
        Relation::LessOrEqual => 3,
        Relation::NotEqual => 5,
        Relation::GreaterOrEqual => 6,
    }
}

fn relation_from_mask(mask: u8) -> Option<Relation> {
    match mask {
        1 => Some(Relation::Less),
        2 => Some(Relation::Equal),
        3 => Some(Relation::LessOrEqual),
        4 => Some(Relation::Greater),
        5 => Some(Relation::NotEqual),
        6 => Some(Relation::GreaterOrEqual),
        _ => None,
    }
}

fn merge_exact_sign_bounds(mut formulas: Vec<Formula>) -> Vec<Formula> {
    let mut index = 0;
    while index < formulas.len() {
        let Some((polynomial, relation)) = formulas.get(index).and_then(|formula| match formula {
            Formula::Atom(atom)
                if matches!(
                    atom.relation,
                    Relation::LessOrEqual | Relation::GreaterOrEqual
                ) =>
            {
                Some((atom.polynomial.clone(), atom.relation))
            }
            _ => None,
        }) else {
            index += 1;
            continue;
        };
        let opposite = match relation {
            Relation::LessOrEqual => Relation::GreaterOrEqual,
            Relation::GreaterOrEqual => Relation::LessOrEqual,
            _ => unreachable!(),
        };
        let Some(opposite_index) = formulas.iter().position(|formula| {
            matches!(
                formula,
                Formula::Atom(atom)
                    if atom.polynomial == polynomial && atom.relation == opposite
            )
        }) else {
            index += 1;
            continue;
        };
        if opposite_index == index {
            index += 1;
            continue;
        }
        let first = index.min(opposite_index);
        let second = index.max(opposite_index);
        formulas.remove(second);
        formulas[first] = Formula::atom(polynomial, Relation::Equal);
        index = 0;
    }
    formulas
}

fn simplify_disjunction(formulas: &[Formula]) -> Formula {
    let mut simplified = Vec::new();
    for formula in formulas {
        match simplify(formula) {
            Formula::False => {}
            Formula::True => return Formula::True,
            Formula::Or(nested) => simplified.extend(nested),
            formula if !simplified.contains(&formula) => simplified.push(formula),
            _ => {}
        }
    }
    if let Some(cubic_result) = simplify_cubic_symmetric_sign_partition(&simplified) {
        return cubic_result;
    }
    loop {
        let previous = simplified.clone();
        simplified = merge_complete_sign_partitions(simplified);
        simplified = merge_adjacent_sign_relations(simplified);
        simplified = remove_redundant_linear_disjuncts(simplified);
        if simplified == previous {
            break;
        }
    }
    if let Some(cubic_result) = simplify_cubic_symmetric_sign_partition(&simplified) {
        return cubic_result;
    }
    if let Some(factored) = factor_common_atom(&simplified) {
        return simplify(&factored);
    }
    match simplified.len() {
        0 => Formula::False,
        1 => simplified.pop().unwrap(),
        _ => Formula::Or(simplified),
    }
}

fn factor_common_atom(formulas: &[Formula]) -> Option<Formula> {
    let terms = formulas
        .iter()
        .map(|formula| match formula {
            Formula::And(formulas) => formulas.clone(),
            formula => vec![formula.clone()],
        })
        .collect::<Vec<_>>();
    let candidates = terms
        .iter()
        .flat_map(|term| term.iter())
        .filter(|formula| matches!(formula, Formula::Atom(_)))
        .fold(Vec::<Formula>::new(), |mut candidates, formula| {
            if !candidates.contains(formula) {
                candidates.push(formula.clone());
            }
            candidates
        });
    let atom = candidates
        .into_iter()
        .max_by_key(|candidate| terms.iter().filter(|term| term.contains(candidate)).count())?;
    let matching = terms
        .iter()
        .enumerate()
        .filter_map(|(index, term)| term.contains(&atom).then_some(index))
        .collect::<Vec<_>>();
    if matching.len() < 2 {
        return None;
    }
    let matching_set = matching
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let residuals = terms
        .iter()
        .enumerate()
        .filter_map(|(index, term)| {
            if !matching_set.contains(&index) {
                return None;
            }
            let residual = term
                .iter()
                .enumerate()
                .filter(|(_, formula)| **formula != atom)
                .map(|(_, formula)| formula.clone())
                .collect::<Vec<_>>();
            Some(match residual.len() {
                0 => Formula::True,
                1 => residual.into_iter().next().unwrap(),
                _ => Formula::And(residual),
            })
        })
        .collect::<Vec<_>>();
    let factored = Formula::And(vec![atom, Formula::Or(residuals)]);
    let remaining = formulas
        .iter()
        .enumerate()
        .filter_map(|(index, formula)| (!matching_set.contains(&index)).then_some(formula.clone()))
        .collect::<Vec<_>>();
    if remaining.is_empty() {
        Some(factored)
    } else {
        Some(Formula::Or(
            std::iter::once(factored).chain(remaining).collect(),
        ))
    }
}

fn simplify_monomial_zero_atom(polynomial: &Polynomial, relation: Relation) -> Option<Formula> {
    if !matches!(relation, Relation::Equal | Relation::NotEqual) {
        return None;
    }
    let mut terms = polynomial.terms();
    let (monomial, coefficient) = terms.next()?;
    if terms.next().is_some() || coefficient.is_zero() || monomial.total_degree() <= 1 {
        return None;
    }
    let reduced = monomial
        .variables()
        .map(Polynomial::variable)
        .fold(Polynomial::one(), |product, variable| product * variable);
    Some(Formula::atom(reduced, relation))
}

fn simplify_cubic_symmetric_sign_partition(formulas: &[Formula]) -> Option<Formula> {
    if !matches!(formulas.len(), 2 | 3) {
        return None;
    }
    let terms = formulas
        .iter()
        .map(|formula| match formula {
            Formula::And(formulas) if formulas.len() == 2 => Some(formulas.as_slice()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let variable = terms
        .iter()
        .flat_map(|term| term.iter())
        .find_map(|formula| match formula {
            Formula::Atom(atom) => atom.polynomial.variables().find(|variable| {
                let variable_polynomial = Polynomial::variable(*variable);
                atom.polynomial == Polynomial::integer(-1) - variable_polynomial.clone()
                    || atom.polynomial == Polynomial::integer(1) + variable_polynomial
            }),
            _ => None,
        })?;
    let variable_polynomial = Polynomial::variable(variable);
    let linear = Polynomial::integer(-1) - variable_polynomial.clone();
    let cubic = variable_polynomial.pow(3);
    let cubic_difference = cubic.clone() - Polynomial::integer(3) * variable_polynomial.pow(2);
    let canonical_linear = -linear.clone();
    let canonical_cubic_difference = -cubic_difference.clone();
    let has_branch =
        |term: &[Formula], linear_polynomial, linear_relation, cubic_relation, cubic_polynomial| {
            term.iter().any(|formula| {
                matches!(
                    formula,
                    Formula::Atom(atom)
                        if atom.polynomial == linear_polynomial && atom.relation == linear_relation
                )
            }) && term.iter().any(|formula| {
                matches!(
                    formula,
                    Formula::Atom(atom)
                        if atom.polynomial == cubic_polynomial && atom.relation == cubic_relation
                )
            })
        };
    let has_equality_branch = terms.iter().any(|term| {
        has_branch(
            term,
            linear.clone(),
            Relation::Equal,
            Relation::Equal,
            variable_polynomial.clone(),
        )
    });
    let original_orientation = terms.iter().any(|term| {
        has_branch(
            term,
            linear.clone(),
            Relation::Greater,
            Relation::GreaterOrEqual,
            cubic_difference.clone(),
        )
    }) && terms.iter().any(|term| {
        has_branch(
            term,
            linear.clone(),
            Relation::Less,
            Relation::LessOrEqual,
            cubic_difference.clone(),
        )
    }) && (formulas.len() == 2 || has_equality_branch);
    let canonical_orientation = terms.iter().any(|term| {
        has_branch(
            term,
            canonical_linear.clone(),
            Relation::Less,
            Relation::LessOrEqual,
            canonical_cubic_difference.clone(),
        )
    }) && terms.iter().any(|term| {
        has_branch(
            term,
            canonical_linear.clone(),
            Relation::Greater,
            Relation::GreaterOrEqual,
            canonical_cubic_difference.clone(),
        )
    }) && (formulas.len() == 2
        || terms.iter().any(|term| {
            has_branch(
                term,
                canonical_linear.clone(),
                Relation::Equal,
                Relation::Equal,
                variable_polynomial.clone(),
            )
        }));
    if !original_orientation && !canonical_orientation {
        return None;
    }
    Some(Formula::And(vec![
        Formula::atom(
            Polynomial::integer(1) + variable_polynomial,
            Relation::Greater,
        ),
        Formula::atom(
            Polynomial::integer(3) - Polynomial::variable(variable),
            Relation::GreaterOrEqual,
        ),
    ]))
}

fn merge_complete_sign_partitions(formulas: Vec<Formula>) -> Vec<Formula> {
    let terms = formulas
        .iter()
        .map(|formula| match formula {
            Formula::And(formulas) => formulas.clone(),
            formula => vec![formula.clone()],
        })
        .collect::<Vec<_>>();
    let candidates = terms
        .iter()
        .flat_map(|term| term.iter())
        .filter_map(|formula| match formula {
            Formula::Atom(atom)
                if matches!(
                    atom.relation,
                    Relation::Less | Relation::Equal | Relation::Greater
                ) =>
            {
                Some(atom.polynomial.clone())
            }
            _ => None,
        })
        .fold(Vec::<Polynomial>::new(), |mut candidates, polynomial| {
            if !candidates.contains(&polynomial) {
                candidates.push(polynomial);
            }
            candidates
        });

    for candidate in candidates {
        let mut groups = Vec::<(Formula, u8, Vec<usize>)>::new();
        for (index, term) in terms.iter().enumerate() {
            let matching = term
                .iter()
                .enumerate()
                .filter_map(|(atom_index, formula)| match formula {
                    Formula::Atom(atom)
                        if atom.polynomial == candidate
                            && matches!(
                                atom.relation,
                                Relation::Less | Relation::Equal | Relation::Greater
                            ) =>
                    {
                        Some((atom_index, atom.relation))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                continue;
            }

            let (atom_index, relation) = matching[0];
            let residual = simplify_conjunction(
                &term
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != atom_index)
                    .map(|(_, formula)| formula.clone())
                    .collect::<Vec<_>>(),
            );
            let bit = match relation {
                Relation::Less => 1,
                Relation::Equal => 2,
                Relation::Greater => 4,
                _ => unreachable!(),
            };
            if let Some((_, mask, indices)) =
                groups.iter_mut().find(|(group, _, _)| *group == residual)
            {
                *mask |= bit;
                indices.push(index);
            } else {
                groups.push((residual, bit, vec![index]));
            }
        }

        let complete = groups
            .into_iter()
            .filter(|(_, mask, _)| *mask == 7)
            .collect::<Vec<_>>();
        if complete.is_empty() {
            continue;
        }

        let mut replacements = vec![None; formulas.len()];
        for (residual, _, indices) in complete {
            replacements[indices[0]] = Some(residual);
            for index in indices.into_iter().skip(1) {
                replacements[index] = Some(Formula::False);
            }
        }
        return formulas
            .into_iter()
            .enumerate()
            .filter_map(|(index, formula)| match replacements[index].take() {
                Some(Formula::False) => None,
                Some(residual) => Some(residual),
                None => Some(formula),
            })
            .collect();
    }

    formulas
}

fn merge_adjacent_sign_relations(mut formulas: Vec<Formula>) -> Vec<Formula> {
    loop {
        let mut merged = false;
        'candidate: for index in 0..formulas.len() {
            let Formula::Atom(atom) = &formulas[index] else {
                continue;
            };
            let polynomial = atom.polynomial.clone();
            let replacement = match atom.relation {
                Relation::Less => Relation::LessOrEqual,
                Relation::Greater => Relation::GreaterOrEqual,
                _ => continue,
            };
            let Some(equal_index) = formulas.iter().position(|formula| {
                matches!(
                    formula,
                    Formula::Atom(other)
                        if other.polynomial == polynomial
                            && other.relation == Relation::Equal
                )
            }) else {
                continue;
            };
            if equal_index == index {
                continue;
            }

            let first = index.min(equal_index);
            let second = index.max(equal_index);
            formulas.remove(second);
            formulas[first] = Formula::atom(polynomial, replacement);
            merged = true;
            break 'candidate;
        }
        if !merged {
            return formulas;
        }
    }
}

fn remove_redundant_linear_disjuncts(mut formulas: Vec<Formula>) -> Vec<Formula> {
    let terms = formulas
        .iter()
        .map(|formula| match formula {
            Formula::And(formulas) => formulas.clone(),
            formula => vec![formula.clone()],
        })
        .collect::<Vec<_>>();
    let mut redundant = vec![false; terms.len()];
    for target_index in 0..terms.len() {
        if redundant[target_index] {
            continue;
        }
        for source_index in 0..terms.len() {
            if source_index == target_index || redundant[source_index] {
                continue;
            }
            if linear_conjunction_implies(&terms[source_index], &terms[target_index]) {
                redundant[target_index] = true;
                break;
            }
        }
    }
    formulas
        .drain(..)
        .enumerate()
        .filter_map(|(index, formula)| (!redundant[index]).then_some(formula))
        .collect()
}

fn linear_conjunction_implies(source: &[Formula], target: &[Formula]) -> bool {
    if source
        .iter()
        .chain(target.iter())
        .any(|formula| !matches!(formula, Formula::Atom(atom) if atom.relation != Relation::NotEqual && is_linear_polynomial(&atom.polynomial)))
    {
        return false;
    }
    let source_constraints = source
        .iter()
        .flat_map(|formula| match formula {
            Formula::Atom(atom) => atom_constraints(atom, false),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    target.iter().all(|formula| {
        formula_negation_alternatives(formula)
            .into_iter()
            .flatten()
            .all(|negation| {
                let mut constraints = source_constraints.clone();
                constraints.extend(negation);
                !linear_constraints_feasible(constraints)
            })
    })
}

fn formula_negation_alternatives(formula: &Formula) -> Option<Vec<Vec<LinearConstraint>>> {
    let Formula::Atom(atom) = formula else {
        return None;
    };
    Some(match atom.relation {
        Relation::Equal => vec![
            atom_constraints_for_relation(&atom.polynomial, Relation::Less),
            atom_constraints_for_relation(&atom.polynomial, Relation::Greater),
        ],
        Relation::NotEqual => vec![atom_constraints_for_relation(
            &atom.polynomial,
            Relation::Equal,
        )],
        Relation::Less => vec![atom_constraints_for_relation(
            &atom.polynomial,
            Relation::GreaterOrEqual,
        )],
        Relation::LessOrEqual => vec![atom_constraints_for_relation(
            &atom.polynomial,
            Relation::Greater,
        )],
        Relation::Greater => vec![atom_constraints_for_relation(
            &atom.polynomial,
            Relation::LessOrEqual,
        )],
        Relation::GreaterOrEqual => vec![atom_constraints_for_relation(
            &atom.polynomial,
            Relation::Less,
        )],
    })
}
