use crate::polynomial::Polynomial;
use std::collections::BTreeSet;

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
}
