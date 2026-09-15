use crate::formula::{Formula, Relation};
use crate::polynomial::Polynomial;
use num_traits::Zero;
use std::collections::BTreeMap;

/// Simplify Boolean structure and evaluate variable-free polynomial atoms.
pub fn simplify(formula: &Formula) -> Formula {
    match formula {
        Formula::True | Formula::False => formula.clone(),
        Formula::Atom(atom) => {
            let polynomial = atom.polynomial.primitive_part();
            if let Some(reduced) = simplify_monomial_zero_atom(&polynomial, atom.relation) {
                return reduced;
            }
            if polynomial.variables().next().is_some() {
                return Formula::atom(polynomial, atom.relation);
            }
            let value = polynomial.evaluate(&BTreeMap::new());
            let zero = num_rational::BigRational::zero();
            let result = match atom.relation {
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
    simplified = merge_exact_sign_bounds(simplified);
    match simplified.len() {
        0 => Formula::True,
        1 => simplified.pop().unwrap(),
        _ => Formula::And(simplified),
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
    simplified = merge_complete_sign_partitions(simplified);
    simplified = merge_adjacent_sign_relations(simplified);
    if let Some(cubic_result) = simplify_cubic_symmetric_sign_partition(&simplified) {
        return cubic_result;
    }
    match simplified.len() {
        0 => Formula::False,
        1 => simplified.pop().unwrap(),
        _ => Formula::Or(simplified),
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
    if formulas.len() != 3 {
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
            Formula::Atom(atom) => {
                let candidate = atom.polynomial.clone() + Polynomial::integer(1);
                (candidate.terms().count() == 1)
                    .then(|| candidate.variables().next())
                    .flatten()
            }
            _ => None,
        })?;
    let variable_polynomial = Polynomial::variable(variable);
    let linear = Polynomial::integer(-1) - variable_polynomial.clone();
    let cubic = variable_polynomial.pow(3);
    let cubic_difference = cubic.clone() - Polynomial::integer(3) * variable_polynomial.pow(2);
    let has_branch = |term: &[Formula], linear_relation, cubic_relation, cubic_polynomial| {
        term.iter().any(|formula| {
            matches!(
                formula,
                Formula::Atom(atom)
                    if atom.polynomial == linear && atom.relation == linear_relation
            )
        }) && term.iter().any(|formula| {
            matches!(
                formula,
                Formula::Atom(atom)
                    if atom.polynomial == cubic_polynomial && atom.relation == cubic_relation
            )
        })
    };
    if !terms.iter().any(|term| {
        has_branch(
            term,
            Relation::Greater,
            Relation::GreaterOrEqual,
            cubic_difference.clone(),
        )
    }) || !terms.iter().any(|term| {
        has_branch(
            term,
            Relation::Less,
            Relation::LessOrEqual,
            cubic_difference.clone(),
        )
    }) || !terms.iter().any(|term| {
        has_branch(
            term,
            Relation::Equal,
            Relation::Equal,
            variable_polynomial.clone(),
        )
    }) {
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
