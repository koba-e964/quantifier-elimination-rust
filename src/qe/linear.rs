use crate::formula::{Atom, Formula, Relation};
use crate::polynomial::Polynomial;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use std::collections::BTreeMap;

const NEGATIVE: u8 = 1;
const ZERO: u8 = 2;
const POSITIVE: u8 = 4;

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

pub(crate) fn merge_semantically_adjacent_linear_disjuncts(
    mut formulas: Vec<Formula>,
) -> Vec<Formula> {
    loop {
        let terms = formulas
            .iter()
            .map(|formula| match formula {
                Formula::And(formulas) => formulas.clone(),
                formula => vec![formula.clone()],
            })
            .collect::<Vec<_>>();
        let mut replacement = None;
        'pairs: for left_index in 0..terms.len() {
            for right_index in (left_index + 1)..terms.len() {
                let left = &terms[left_index];
                let right = &terms[right_index];
                if left
                    .iter()
                    .chain(right.iter())
                    .any(|formula| !is_linear_atom(formula))
                {
                    continue;
                }
                let candidates = left
                    .iter()
                    .chain(right.iter())
                    .filter(|formula| {
                        linear_conjunction_implies(left, std::slice::from_ref(formula))
                            && linear_conjunction_implies(right, std::slice::from_ref(formula))
                    })
                    .fold(Vec::<Formula>::new(), |mut candidates, formula| {
                        if !candidates.contains(formula) {
                            candidates.push(formula.clone());
                        }
                        candidates
                    });
                if candidates.is_empty()
                    || candidates.len() >= left.len().max(right.len())
                    || !linear_union_covers(&candidates, &[left, right])
                {
                    continue;
                }
                replacement = Some((left_index, right_index, candidates));
                break 'pairs;
            }
        }
        let Some((left_index, right_index, candidates)) = replacement else {
            return formulas;
        };
        formulas.remove(right_index);
        formulas[left_index] = match candidates.len() {
            1 => candidates.into_iter().next().unwrap(),
            _ => Formula::And(candidates),
        };
    }
}

/// Minimize a finite disjunction of linear sign regions without changing its truth set.
pub(crate) fn minimize_linear_disjunction(formulas: Vec<Formula>) -> Option<Vec<Formula>> {
    let terms = formulas
        .iter()
        .map(|formula| match formula {
            Formula::And(formulas) => formulas.clone(),
            formula => vec![formula.clone()],
        })
        .collect::<Vec<_>>();
    if terms
        .iter()
        .flatten()
        .any(|formula| !is_linear_atom(formula))
    {
        return None;
    }
    let polynomials = terms
        .iter()
        .flatten()
        .filter_map(|formula| match formula {
            Formula::Atom(atom) => Some(atom.polynomial.clone()),
            _ => None,
        })
        .fold(Vec::<Polynomial>::new(), |mut polynomials, polynomial| {
            if !polynomials.contains(&polynomial) {
                polynomials.push(polynomial);
            }
            polynomials
        });
    if polynomials.is_empty() || polynomials.len() > 8 {
        return None;
    }
    let mut assignments = Vec::new();
    enumerate_sign_cells(&polynomials, &mut Vec::new(), &mut assignments);
    let feasible = assignments
        .into_iter()
        .filter(|assignment| sign_assignment_feasible(&polynomials, assignment))
        .collect::<Vec<_>>();
    if feasible.is_empty() {
        return None;
    }
    let truth = feasible
        .iter()
        .map(|assignment| sign_assignment_satisfies(&terms, &polynomials, assignment))
        .collect::<Vec<_>>();
    let true_count = truth.iter().filter(|value| **value).count();
    if true_count == 0 || true_count == feasible.len() {
        return None;
    }
    let mut cubes = Vec::new();
    for (index, assignment) in feasible.iter().enumerate() {
        if !truth[index] {
            continue;
        }
        let mut cube = assignment.clone();
        for polynomial_index in 0..cube.len() {
            let current = cube[polynomial_index];
            let mut options = [
                NEGATIVE | ZERO | POSITIVE,
                NEGATIVE | ZERO,
                ZERO | POSITIVE,
                NEGATIVE | POSITIVE,
            ]
            .into_iter()
            .filter(|mask| mask & current == current)
            .collect::<Vec<_>>();
            options.sort_by_key(|mask| std::cmp::Reverse(mask.count_ones()));
            for mask in options {
                cube[polynomial_index] = mask;
                if cube_is_true(&cube, &feasible, &truth) {
                    break;
                }
                cube[polynomial_index] = current;
            }
        }
        if !cubes.contains(&cube) {
            cubes.push(cube);
        }
    }
    cubes.sort_by_key(|cube| {
        std::cmp::Reverse(cube.iter().map(|mask| mask.count_ones()).sum::<u32>())
    });
    let mut covered = vec![false; feasible.len()];
    let mut selected = Vec::new();
    while covered
        .iter()
        .zip(&truth)
        .any(|(is_covered, is_true)| *is_true && !is_covered)
    {
        let (cube_index, _) = cubes
            .iter()
            .enumerate()
            .map(|(cube_index, cube)| {
                let gain = feasible
                    .iter()
                    .enumerate()
                    .filter(|(index, assignment)| {
                        truth[*index] && !covered[*index] && cube_matches(cube, assignment)
                    })
                    .count();
                (cube_index, gain)
            })
            .max_by_key(|(_, gain)| *gain)?;
        if selected.contains(&cube_index) {
            return None;
        }
        let cube = &cubes[cube_index];
        let gain = feasible
            .iter()
            .enumerate()
            .filter(|(index, assignment)| {
                truth[*index] && !covered[*index] && cube_matches(cube, assignment)
            })
            .count();
        if gain == 0 {
            return None;
        }
        for (index, assignment) in feasible.iter().enumerate() {
            if cube_matches(cube, assignment) {
                covered[index] = true;
            }
        }
        selected.push(cube_index);
    }
    let result = selected
        .into_iter()
        .map(|index| cube_to_formula(&polynomials, &cubes[index]))
        .collect::<Vec<_>>();
    (result != formulas).then_some(result)
}

fn enumerate_sign_cells(
    polynomials: &[Polynomial],
    prefix: &mut Vec<u8>,
    assignments: &mut Vec<Vec<u8>>,
) {
    if prefix.len() == polynomials.len() {
        assignments.push(prefix.clone());
        return;
    }
    for sign in [NEGATIVE, ZERO, POSITIVE] {
        prefix.push(sign);
        enumerate_sign_cells(polynomials, prefix, assignments);
        prefix.pop();
    }
}

fn sign_assignment_feasible(polynomials: &[Polynomial], assignment: &[u8]) -> bool {
    let constraints = polynomials
        .iter()
        .zip(assignment)
        .flat_map(|(polynomial, sign)| {
            let relation = match *sign {
                NEGATIVE => Relation::Less,
                ZERO => Relation::Equal,
                POSITIVE => Relation::Greater,
                _ => unreachable!(),
            };
            atom_constraints_for_relation(polynomial, relation)
        })
        .collect();
    linear_constraints_feasible(constraints)
}

fn sign_assignment_satisfies(
    terms: &[Vec<Formula>],
    polynomials: &[Polynomial],
    assignment: &[u8],
) -> bool {
    terms.iter().any(|term| {
        term.iter().all(|formula| {
            let Formula::Atom(atom) = formula else {
                return false;
            };
            let sign = assignment[polynomials
                .iter()
                .position(|polynomial| polynomial == &atom.polynomial)
                .unwrap()];
            match atom.relation {
                Relation::Less => sign == NEGATIVE,
                Relation::Equal => sign == ZERO,
                Relation::Greater => sign == POSITIVE,
                Relation::LessOrEqual => sign & (NEGATIVE | ZERO) != 0,
                Relation::NotEqual => sign != ZERO,
                Relation::GreaterOrEqual => sign & (ZERO | POSITIVE) != 0,
            }
        })
    })
}

fn cube_is_true(cube: &[u8], assignments: &[Vec<u8>], truth: &[bool]) -> bool {
    assignments
        .iter()
        .enumerate()
        .filter(|(_, assignment)| cube_matches(cube, assignment))
        .all(|(index, _)| truth[index])
}

fn cube_matches(cube: &[u8], assignment: &[u8]) -> bool {
    cube.iter()
        .zip(assignment)
        .all(|(mask, sign)| mask & sign != 0)
}

fn cube_to_formula(polynomials: &[Polynomial], cube: &[u8]) -> Formula {
    let atoms = polynomials
        .iter()
        .zip(cube)
        .filter_map(|(polynomial, mask)| {
            let relation = match *mask {
                NEGATIVE => Relation::Less,
                ZERO => Relation::Equal,
                POSITIVE => Relation::Greater,
                mask if mask == NEGATIVE | ZERO => Relation::LessOrEqual,
                mask if mask == ZERO | POSITIVE => Relation::GreaterOrEqual,
                mask if mask == NEGATIVE | POSITIVE => Relation::NotEqual,
                _ => return None,
            };
            Some(Formula::atom(polynomial.clone(), relation))
        })
        .collect::<Vec<_>>();
    match atoms.len() {
        0 => Formula::True,
        1 => atoms.into_iter().next().unwrap(),
        _ => Formula::And(atoms),
    }
}

fn is_linear_atom(formula: &Formula) -> bool {
    matches!(
        formula,
        Formula::Atom(atom)
            if atom.relation != Relation::NotEqual && is_linear_polynomial(&atom.polynomial)
    )
}

fn linear_union_covers(candidate: &[Formula], terms: &[&Vec<Formula>]) -> bool {
    let candidate_constraints = candidate
        .iter()
        .flat_map(|formula| match formula {
            Formula::Atom(atom) => atom_constraints(atom, false),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    let mut outside = vec![Vec::new()];
    for term in terms {
        let negations = term
            .iter()
            .filter_map(formula_negation_alternatives)
            .flatten()
            .collect::<Vec<_>>();
        outside = outside
            .into_iter()
            .flat_map(|prefix| {
                negations.iter().map(move |negation| {
                    let mut constraints = prefix.clone();
                    constraints.extend(negation.clone());
                    constraints
                })
            })
            .collect();
    }
    outside.into_iter().all(|negations| {
        let mut constraints = candidate_constraints.clone();
        constraints.extend(negations);
        !linear_constraints_feasible(constraints)
    })
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
