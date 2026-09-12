use crate::formula::{Atom, Formula, Quantifier, Relation};

/// Convert a formula to negation normal form, pushing negations to atoms and
/// dualizing quantifiers along the way.
pub fn to_nnf(formula: &Formula) -> Formula {
    normalize(formula, false)
}

fn normalize(formula: &Formula, negated: bool) -> Formula {
    match formula {
        Formula::True => constant(!negated),
        Formula::False => constant(negated),
        Formula::Atom(atom) => Formula::Atom(Atom::new(
            atom.polynomial.clone(),
            if negated {
                negate_relation(atom.relation)
            } else {
                atom.relation
            },
        )),
        Formula::Not(body) => normalize(body, !negated),
        Formula::And(formulas) => combine(
            formulas
                .iter()
                .map(|formula| normalize(formula, negated))
                .collect(),
            if negated {
                FormulaKind::Or
            } else {
                FormulaKind::And
            },
        ),
        Formula::Or(formulas) => combine(
            formulas
                .iter()
                .map(|formula| normalize(formula, negated))
                .collect(),
            if negated {
                FormulaKind::And
            } else {
                FormulaKind::Or
            },
        ),
        Formula::Quantified {
            quantifier,
            variable,
            body,
        } => Formula::Quantified {
            quantifier: if negated {
                match quantifier {
                    Quantifier::Forall => Quantifier::Exists,
                    Quantifier::Exists => Quantifier::Forall,
                }
            } else {
                *quantifier
            },
            variable: *variable,
            body: Box::new(normalize(body, negated)),
        },
    }
}

enum FormulaKind {
    And,
    Or,
}

fn combine(formulas: Vec<Formula>, kind: FormulaKind) -> Formula {
    match kind {
        FormulaKind::And => Formula::And(formulas),
        FormulaKind::Or => Formula::Or(formulas),
    }
}

fn constant(value: bool) -> Formula {
    if value {
        Formula::True
    } else {
        Formula::False
    }
}

fn negate_relation(relation: Relation) -> Relation {
    match relation {
        Relation::Equal => Relation::NotEqual,
        Relation::NotEqual => Relation::Equal,
        Relation::Less => Relation::GreaterOrEqual,
        Relation::LessOrEqual => Relation::Greater,
        Relation::Greater => Relation::LessOrEqual,
        Relation::GreaterOrEqual => Relation::Less,
    }
}
