//! Exact symbolic building blocks for quantifier elimination over the reals.
//!
//! The crate is intentionally split into a language layer and algorithmic
//! layers.  The initial implementation provides exact rational coefficients,
//! canonical multivariate polynomials, and quantified formula syntax.  CAD
//! elimination is built on these types.

pub mod algebra;
pub mod cad;
pub mod formula;
pub mod parser;
pub mod polynomial;
pub mod qe;

pub use algebra::algebraic::AlgebraicReal;
pub use algebra::coefficient::{
    AlgebraicPolynomial, AlgebraicRootSample, ExactReal, ExactRealError,
};
pub use algebra::univariate::{RootInterval, UnivariatePolynomial};
pub use formula::{Atom, Formula, Quantifier, Relation, RenameError};
pub use parser::{parse_formula, ParseError};
pub use polynomial::{Monomial, Polynomial, PolynomialEvaluationError, Variable, VariableNames};
pub use qe::evaluate::{
    eliminate, eliminate_with_stats, EliminationStats, QuantifierEvaluationError,
};
