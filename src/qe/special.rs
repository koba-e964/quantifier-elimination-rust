use crate::formula::{Atom, Formula, Quantifier, Relation};
use crate::polynomial::Polynomial;
use crate::qe::simplify::simplify;

pub(crate) fn eliminate_symmetric_pair(formula: &Formula) -> Option<Formula> {
    let Formula::Quantified {
        quantifier: Quantifier::Exists,
        variable: left,
        body,
    } = formula
    else {
        return None;
    };
    let Formula::Quantified {
        quantifier: Quantifier::Exists,
        variable: right,
        body,
    } = body.as_ref()
    else {
        return None;
    };
    if left == right {
        return None;
    }
    let atoms = match body.as_ref() {
        Formula::Atom(atom) => vec![atom],
        Formula::And(formulas) => formulas
            .iter()
            .map(|formula| match formula {
                Formula::Atom(atom) => Some(atom),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?,
        _ => return None,
    };
    let free_variables = body
        .free_variables()
        .into_iter()
        .filter(|variable| variable != left && variable != right)
        .collect::<Vec<_>>();
    let sum_aliases = free_variables
        .iter()
        .copied()
        .filter(|alias| {
            atoms
                .iter()
                .any(|atom| is_sum_binding(atom, *left, *right, *alias))
        })
        .collect::<Vec<_>>();
    let product_aliases = free_variables
        .iter()
        .copied()
        .filter(|alias| {
            atoms
                .iter()
                .any(|atom| is_product_binding(atom, *left, *right, *alias))
        })
        .collect::<Vec<_>>();
    let [sum_alias] = sum_aliases.as_slice() else {
        return None;
    };
    let [product_alias] = product_aliases.as_slice() else {
        return None;
    };

    let mut rewritten = Vec::new();
    for atom in atoms {
        if is_sum_binding(atom, *left, *right, *sum_alias)
            || is_product_binding(atom, *left, *right, *product_alias)
        {
            continue;
        }
        let polynomial =
            atom.polynomial
                .rewrite_symmetric(*left, *right, *sum_alias, *product_alias)?;
        rewritten.push(Formula::atom(polynomial, atom.relation));
    }
    let discriminant = Polynomial::variable(*sum_alias).pow(2)
        - Polynomial::variable(*product_alias) * Polynomial::integer(4);
    rewritten.push(Formula::atom(discriminant, Relation::GreaterOrEqual));
    Some(simplify(&Formula::And(rewritten)))
}

fn is_sum_binding(atom: &Atom, left: usize, right: usize, alias: usize) -> bool {
    if atom.relation != Relation::Equal {
        return false;
    }
    let expected =
        Polynomial::variable(alias) - Polynomial::variable(left) - Polynomial::variable(right);
    atom.polynomial == expected || atom.polynomial == -expected
}

fn is_product_binding(atom: &Atom, left: usize, right: usize, alias: usize) -> bool {
    if atom.relation != Relation::Equal {
        return false;
    }
    let expected =
        Polynomial::variable(alias) - Polynomial::variable(left) * Polynomial::variable(right);
    atom.polynomial == expected || atom.polynomial == -expected
}
