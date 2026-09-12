//! Exact symbolic building blocks for quantifier elimination over the reals.
//!
//! The crate is intentionally split into a language layer and algorithmic
//! layers.  The initial implementation provides exact rational coefficients,
//! canonical multivariate polynomials, and quantified formula syntax.  CAD
//! elimination is built on these types.

pub mod algebra;
pub mod cad;
pub mod formula;
pub mod polynomial;
pub mod qe;

pub use algebra::algebraic::AlgebraicReal;
pub use algebra::coefficient::{AlgebraicPolynomial, ExactReal};
pub use algebra::univariate::{RootInterval, UnivariatePolynomial};
pub use formula::{Atom, Formula, Quantifier, Relation, RenameError};
pub use polynomial::{Monomial, Polynomial, Variable, VariableNames};
pub use qe::evaluate::{eliminate, QuantifierEvaluationError};
