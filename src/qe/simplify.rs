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
    match simplified.len() {
        0 => Formula::True,
        1 => simplified.pop().unwrap(),
        _ => Formula::And(simplified),
    }
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
    match simplified.len() {
        0 => Formula::False,
        1 => simplified.pop().unwrap(),
        _ => Formula::Or(simplified),
    }
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
