use super::{algebraic::AlgebraicReal, univariate::UnivariatePolynomial};
use num_rational::BigRational;
use num_traits::Zero;
use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactRealError {
    AlgebraicArithmeticNotImplemented,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactReal {
    Rational(BigRational),
    Algebraic(AlgebraicReal),
}

impl ExactReal {
    pub fn rational(value: BigRational) -> Self {
        Self::Rational(value)
    }

    pub fn algebraic(value: AlgebraicReal) -> Self {
        Self::Algebraic(value)
    }

    pub fn sign(&self) -> Ordering {
        match self {
            Self::Rational(value) => value.cmp(&BigRational::zero()),
            Self::Algebraic(value) => match value.sign_of(&UnivariatePolynomial::new(vec![
                BigRational::zero(),
                BigRational::from_integer(1.into()),
            ])) {
                sign if sign < 0 => Ordering::Less,
                0 => Ordering::Equal,
                _ => Ordering::Greater,
            },
        }
    }

    pub fn try_add(&self, other: &Self) -> Result<Self, ExactRealError> {
        match (self, other) {
            (Self::Rational(left), Self::Rational(right)) => Ok(Self::Rational(left + right)),
            _ => Err(ExactRealError::AlgebraicArithmeticNotImplemented),
        }
    }

    pub fn try_sub(&self, other: &Self) -> Result<Self, ExactRealError> {
        match (self, other) {
            (Self::Rational(left), Self::Rational(right)) => Ok(Self::Rational(left - right)),
            _ => Err(ExactRealError::AlgebraicArithmeticNotImplemented),
        }
    }

    pub fn try_mul(&self, other: &Self) -> Result<Self, ExactRealError> {
        match (self, other) {
            (Self::Rational(left), Self::Rational(right)) => Ok(Self::Rational(left * right)),
            _ => Err(ExactRealError::AlgebraicArithmeticNotImplemented),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgebraicPolynomial {
    coefficients: Vec<ExactReal>,
}

impl AlgebraicPolynomial {
    pub fn new(coefficients: Vec<ExactReal>) -> Self {
        Self { coefficients }
    }

    pub fn coefficients(&self) -> &[ExactReal] {
        &self.coefficients
    }

    pub fn degree(&self) -> Option<usize> {
        (!self.coefficients.is_empty()).then_some(self.coefficients.len() - 1)
    }
}
