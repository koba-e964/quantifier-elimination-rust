use super::univariate::{RootInterval, UnivariatePolynomial};

/// An exact real algebraic number represented by a defining polynomial and an
/// isolating interval containing exactly one of its real roots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgebraicReal {
    pub polynomial: UnivariatePolynomial,
    pub interval: RootInterval,
}

impl AlgebraicReal {
    pub fn new(polynomial: UnivariatePolynomial, interval: RootInterval) -> Self {
        Self {
            polynomial,
            interval,
        }
    }
}
