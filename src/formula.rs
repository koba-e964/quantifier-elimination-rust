use crate::polynomial::Polynomial;
use num_rational::BigRational;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Relation {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Atom {
    pub polynomial: Polynomial,
    pub relation: Relation,
}

impl Atom {
    pub fn new(polynomial: Polynomial, relation: Relation) -> Self {
        Self {
            polynomial,
            relation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Quantifier {
    Forall,
    Exists,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Formula {
    True,
    False,
    Atom(Atom),
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
    Quantified {
        quantifier: Quantifier,
        variable: usize,
        body: Box<Self>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenameError {
    NotBound,
    WouldCapture,
}

impl Formula {
    pub fn atom(polynomial: Polynomial, relation: Relation) -> Self {
        Self::Atom(Atom::new(polynomial, relation))
    }

    pub fn exists(variable: usize, body: Self) -> Self {
        Self::Quantified {
            quantifier: Quantifier::Exists,
            variable,
            body: Box::new(body),
        }
    }

    pub fn forall(variable: usize, body: Self) -> Self {
        Self::Quantified {
            quantifier: Quantifier::Forall,
            variable,
            body: Box::new(body),
        }
    }

    pub fn free_variables(&self) -> BTreeSet<usize> {
        match self {
            Self::True | Self::False => BTreeSet::new(),
            Self::Atom(atom) => atom.polynomial.variables().collect(),
            Self::Not(body) => body.free_variables(),
            Self::And(formulas) | Self::Or(formulas) => {
                formulas.iter().flat_map(Self::free_variables).collect()
            }
            Self::Quantified { variable, body, .. } => {
                let mut variables = body.free_variables();
                variables.remove(variable);
                variables
            }
        }
    }

    pub fn is_quantifier_free(&self) -> bool {
        match self {
            Self::True | Self::False | Self::Atom(_) => true,
            Self::Not(body) => body.is_quantifier_free(),
            Self::And(formulas) | Self::Or(formulas) => {
                formulas.iter().all(Self::is_quantifier_free)
            }
            Self::Quantified { .. } => false,
        }
    }

    pub fn evaluate(&self, values: &BTreeMap<usize, BigRational>) -> Option<bool> {
        match self {
            Self::True => Some(true),
            Self::False => Some(false),
            Self::Atom(atom) => {
                let value = atom.polynomial.try_evaluate(values)?;
                Some(match atom.relation {
                    Relation::Equal => value == BigRational::from_integer(0.into()),
                    Relation::NotEqual => value != BigRational::from_integer(0.into()),
                    Relation::Less => value < BigRational::from_integer(0.into()),
                    Relation::LessOrEqual => value <= BigRational::from_integer(0.into()),
                    Relation::Greater => value > BigRational::from_integer(0.into()),
                    Relation::GreaterOrEqual => value >= BigRational::from_integer(0.into()),
                })
            }
            Self::Not(body) => Some(!body.evaluate(values)?),
            Self::And(formulas) => formulas
                .iter()
                .map(|formula| formula.evaluate(values))
                .try_fold(true, |result, value| Some(result && value?)),
            Self::Or(formulas) => formulas
                .iter()
                .map(|formula| formula.evaluate(values))
                .try_fold(false, |result, value| Some(result || value?)),
            Self::Quantified { .. } => None,
        }
    }

    /// Alpha-rename the outermost binder for `variable`, rejecting capture.
    pub fn alpha_rename(&self, variable: usize, replacement: usize) -> Result<Self, RenameError> {
        let Self::Quantified {
            quantifier,
            variable: bound,
            body,
        } = self
        else {
            return Err(RenameError::NotBound);
        };
        if *bound != variable {
            return Err(RenameError::NotBound);
        }
        if body.free_variables().contains(&replacement) {
            return Err(RenameError::WouldCapture);
        }
        Ok(Self::Quantified {
            quantifier: *quantifier,
            variable: replacement,
            body: Box::new(rename_bound_occurrences(body, variable, replacement)),
        })
    }
}

fn rename_bound_occurrences(formula: &Formula, old: usize, new: usize) -> Formula {
    match formula {
        Formula::True => Formula::True,
        Formula::False => Formula::False,
        Formula::Atom(atom) => Formula::Atom(Atom::new(
            atom.polynomial.rename_variable(old, new),
            atom.relation,
        )),
        Formula::Not(body) => Formula::Not(Box::new(rename_bound_occurrences(body, old, new))),
        Formula::And(formulas) => Formula::And(
            formulas
                .iter()
                .map(|formula| rename_bound_occurrences(formula, old, new))
                .collect(),
        ),
        Formula::Or(formulas) => Formula::Or(
            formulas
                .iter()
                .map(|formula| rename_bound_occurrences(formula, old, new))
                .collect(),
        ),
        Formula::Quantified {
            quantifier,
            variable,
            body,
        } if *variable == old => Formula::Quantified {
            quantifier: *quantifier,
            variable: *variable,
            body: body.clone(),
        },
        Formula::Quantified {
            quantifier,
            variable,
            body,
        } => Formula::Quantified {
            quantifier: *quantifier,
            variable: *variable,
            body: Box::new(rename_bound_occurrences(body, old, new)),
        },
    }
}
