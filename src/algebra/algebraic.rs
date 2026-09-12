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

    pub fn negated(&self) -> Self {
        let coefficients = (0..=self.polynomial.degree().unwrap_or(0))
            .map(|degree| {
                let coefficient = self.polynomial.coefficient(degree);
                if degree % 2 == 0 {
                    coefficient
                } else {
                    -coefficient
                }
            })
            .collect();
        Self::new(
            UnivariatePolynomial::new(coefficients),
            RootInterval::new(-self.interval.upper.clone(), -self.interval.lower.clone()),
        )
    }

    pub fn add_rational(&self, value: &num_rational::BigRational) -> Self {
        let transformed = affine_transform(
            &self.polynomial,
            value,
            &num_rational::BigRational::from_integer(1.into()),
        );
        Self::new(
            transformed,
            RootInterval::new(&self.interval.lower + value, &self.interval.upper + value),
        )
    }

    pub fn mul_rational(&self, value: &num_rational::BigRational) -> Option<Self> {
        if value.is_zero() {
            return None;
        }
        let transformed = affine_transform(
            &self.polynomial,
            &num_rational::BigRational::zero(),
            &(num_rational::BigRational::from_integer(1.into()) / value),
        );
        let lower = &self.interval.lower * value;
        let upper = &self.interval.upper * value;
        Some(Self::new(
            transformed,
            RootInterval::new(lower.clone().min(upper.clone()), lower.max(upper)),
        ))
    }

    pub fn compare_rational(&self, value: &num_rational::BigRational) -> Ordering {
        let polynomial = UnivariatePolynomial::new(vec![
            -value.clone(),
            num_rational::BigRational::from_integer(1.into()),
        ]);
        match self.sign_of(&polynomial) {
            sign if sign < 0 => Ordering::Less,
            0 => Ordering::Equal,
            _ => Ordering::Greater,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.sign_of(&UnivariatePolynomial::new(vec![
            num_rational::BigRational::zero(),
            num_rational::BigRational::from_integer(1.into()),
        ])) == 0
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

fn affine_transform(
    polynomial: &UnivariatePolynomial,
    shift: &num_rational::BigRational,
    scale: &num_rational::BigRational,
) -> UnivariatePolynomial {
    let mut result = UnivariatePolynomial::zero();
    for degree in (0..=polynomial.degree().unwrap_or(0)).rev() {
        let transformed = UnivariatePolynomial::new(vec![-shift.clone(), scale.clone()]);
        result = multiply(&result, &transformed)
            + UnivariatePolynomial::constant(polynomial.coefficient(degree));
    }
    result
}

fn multiply(left: &UnivariatePolynomial, right: &UnivariatePolynomial) -> UnivariatePolynomial {
    let mut coefficients = vec![
        num_rational::BigRational::zero();
        left.degree().unwrap_or(0) + right.degree().unwrap_or(0) + 1
    ];
    for left_degree in 0..=left.degree().unwrap_or(0) {
        for right_degree in 0..=right.degree().unwrap_or(0) {
            coefficients[left_degree + right_degree] +=
                left.coefficient(left_degree) * right.coefficient(right_degree);
        }
    }
    UnivariatePolynomial::new(coefficients)
}
