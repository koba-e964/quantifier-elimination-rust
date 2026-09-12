use super::univariate::{RootInterval, UnivariatePolynomial};
use num_bigint::BigInt;
use num_traits::{Signed, Zero};
use std::cmp::Ordering;

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

    pub fn compare(&self, other: &Self) -> Ordering {
        let mut left = self.clone();
        let mut right = other.clone();
        loop {
            if left.interval.upper < right.interval.lower {
                return Ordering::Less;
            }
            if right.interval.upper < left.interval.lower {
                return Ordering::Greater;
            }
            let lower = left
                .interval
                .lower
                .clone()
                .max(right.interval.lower.clone());
            let upper = left
                .interval
                .upper
                .clone()
                .min(right.interval.upper.clone());
            if lower < upper
                && left
                    .polynomial
                    .gcd(&right.polynomial)
                    .count_roots(&RootInterval::new(lower, upper))
                    > 0
            {
                return Ordering::Equal;
            }
            let width = left.interval.width().min(right.interval.width()) / BigInt::from(2);
            left = left.refine(&width);
            right = right.refine(&width);
        }
    }
}
