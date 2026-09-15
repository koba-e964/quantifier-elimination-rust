use super::linear::remove_redundant_linear_disjuncts;
use super::simplify::{simplify, simplify_conjunction};
use crate::formula::{Formula, Relation};
use crate::polynomial::Polynomial;
use num_traits::Zero;

pub(crate) fn merge_sign_constraints(formulas: &mut Vec<Formula>) -> bool {
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

pub(crate) fn merge_exact_sign_bounds(mut formulas: Vec<Formula>) -> Vec<Formula> {
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

pub(crate) fn simplify_disjunction(formulas: &[Formula]) -> Formula {
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

pub(crate) fn simplify_monomial_zero_atom(
    polynomial: &Polynomial,
    relation: Relation,
) -> Option<Formula> {
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
