use crate::formula::{Atom, Formula, Relation};
use crate::polynomial::Polynomial;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
enum LinearBound {
    Lower { value: BigRational, strict: bool },
    Upper { value: BigRational, strict: bool },
    Point(BigRational),
}

pub(crate) fn simplify_univariate_linear_bounds(formulas: &mut Vec<Formula>) -> bool {
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

pub(crate) fn simplify_redundant_linear_constraints(formulas: &mut Vec<Formula>) {
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
            if others.iter().any(|formula| {
                !matches!(formula, Formula::Atom(atom) if atom.relation != Relation::NotEqual && is_linear_polynomial(&atom.polynomial))
            }) {
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

pub(crate) fn linear_conjunction_is_infeasible(formulas: &[Formula]) -> bool {
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

fn linear_atom_implied(others: &[Formula], target: &Atom) -> bool {
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

fn atom_constraints(atom: &Atom, negate: bool) -> Option<Vec<LinearConstraint>> {
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

pub(crate) fn remove_redundant_linear_disjuncts(mut formulas: Vec<Formula>) -> Vec<Formula> {
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
