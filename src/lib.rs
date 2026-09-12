//! Exact symbolic building blocks for quantifier elimination over the reals.
//!
//! The crate is intentionally split into a language layer and algorithmic
//! layers.  The initial implementation provides exact rational coefficients,
//! canonical multivariate polynomials, and quantified formula syntax.  CAD
//! elimination is built on these types.

pub mod algebra;
pub mod formula;
pub mod polynomial;

pub use algebra::univariate::{RootInterval, UnivariatePolynomial};
pub use formula::{Atom, Formula, Quantifier, Relation};
pub use polynomial::{Monomial, Polynomial, Variable};
