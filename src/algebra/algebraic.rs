use super::univariate::{RootInterval, UnivariatePolynomial};
use num_bigint::BigInt;
use num_traits::{Signed, Zero};

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

    pub fn refine(&self, maximum_width: &num_rational::BigRational) -> Self {
        Self {
            polynomial: self.polynomial.clone(),
            interval: self.polynomial.refine_root(&self.interval, maximum_width),
        }
    }

    pub fn rational_value(&self) -> Option<num_rational::BigRational> {
        let midpoint = (&self.interval.lower + &self.interval.upper) / num_bigint::BigInt::from(2);
        (self.polynomial.evaluate(&midpoint).is_zero()).then_some(midpoint)
    }

    pub fn sign_of(&self, polynomial: &UnivariatePolynomial) -> i8 {
        let common = self.polynomial.gcd(polynomial);
        if common.count_roots(&self.interval) > 0 {
            return 0;
        }

        let mut interval = self.interval.clone();
        loop {
            if polynomial.count_roots(&interval) == 0 {
                let value =
                    polynomial.evaluate(&((&interval.lower + &interval.upper) / BigInt::from(2)));
                return if value.is_positive() { 1 } else { -1 };
            }
            interval = self
                .polynomial
                .refine_root(&interval, &(interval.width() / BigInt::from(2)));
        }
    }
}
