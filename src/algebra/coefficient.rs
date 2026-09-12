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

    pub fn is_zero(&self) -> bool {
        self.sign() == Ordering::Equal
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
            (Self::Algebraic(left), Self::Rational(right)) => {
                Ok(Self::Algebraic(left.add_rational(right)))
            }
            (Self::Rational(left), Self::Algebraic(right)) => {
                Ok(Self::Algebraic(right.add_rational(left)))
            }
            (Self::Algebraic(left), Self::Algebraic(right)) => left
                .add_algebraic(right)
                .map(Self::Algebraic)
                .ok_or(ExactRealError::AlgebraicArithmeticNotImplemented),
        }
    }

    pub fn try_sub(&self, other: &Self) -> Result<Self, ExactRealError> {
        match (self, other) {
            (Self::Rational(left), Self::Rational(right)) => Ok(Self::Rational(left - right)),
            (Self::Algebraic(left), Self::Rational(right)) => {
                Ok(Self::Algebraic(left.add_rational(&-right)))
            }
            (Self::Rational(left), Self::Algebraic(right)) => {
                Ok(Self::Algebraic(right.negated().add_rational(left)))
            }
            (Self::Algebraic(left), Self::Algebraic(right)) if left == right => {
                Ok(Self::Rational(BigRational::zero()))
            }
            (Self::Algebraic(left), Self::Algebraic(right)) => left
                .add_algebraic(&right.negated())
                .map(Self::Algebraic)
                .ok_or(ExactRealError::AlgebraicArithmeticNotImplemented),
        }
    }

    pub fn try_mul(&self, other: &Self) -> Result<Self, ExactRealError> {
        match (self, other) {
            (Self::Rational(left), Self::Rational(right)) => Ok(Self::Rational(left * right)),
            (Self::Algebraic(left), Self::Rational(right)) => match left.mul_rational(right) {
                Some(value) => Ok(Self::Algebraic(value)),
                None => Ok(Self::Rational(BigRational::zero())),
            },
            (Self::Rational(left), Self::Algebraic(right)) => match right.mul_rational(left) {
                Some(value) => Ok(Self::Algebraic(value)),
                None => Ok(Self::Rational(BigRational::zero())),
            },
            (Self::Algebraic(left), Self::Algebraic(right)) => left
                .mul_algebraic(right)
                .map(Self::Algebraic)
                .ok_or(ExactRealError::AlgebraicArithmeticNotImplemented),
        }
    }

    pub fn negated(&self) -> Self {
        match self {
            Self::Rational(value) => Self::Rational(-value),
            Self::Algebraic(value) => Self::Algebraic(value.negated()),
        }
    }

    pub fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Rational(left), Self::Rational(right)) => left.cmp(right),
            (Self::Algebraic(left), Self::Rational(right)) => left.compare_rational(right),
            (Self::Rational(left), Self::Algebraic(right)) => {
                right.compare_rational(left).reverse()
            }
            (Self::Algebraic(left), Self::Algebraic(right)) => left.compare(right),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgebraicPolynomial {
    coefficients: Vec<ExactReal>,
}

impl AlgebraicPolynomial {
    pub fn zero() -> Self {
        Self {
            coefficients: Vec::new(),
        }
    }

    pub fn new(mut coefficients: Vec<ExactReal>) -> Self {
        while coefficients.last().is_some_and(ExactReal::is_zero) {
            coefficients.pop();
        }
        Self { coefficients }
    }

    pub fn coefficients(&self) -> &[ExactReal] {
        &self.coefficients
    }

    pub fn degree(&self) -> Option<usize> {
        (!self.coefficients.is_empty()).then_some(self.coefficients.len() - 1)
    }

    pub fn coefficient(&self, degree: usize) -> ExactReal {
        self.coefficients
            .get(degree)
            .cloned()
            .unwrap_or_else(|| ExactReal::rational(BigRational::zero()))
    }

    pub fn derivative(&self) -> Result<Self, ExactRealError> {
        if self.coefficients.len() < 2 {
            return Ok(Self::zero());
        }
        let coefficients = self
            .coefficients
            .iter()
            .enumerate()
            .skip(1)
            .map(|(degree, coefficient)| {
                coefficient.try_mul(&ExactReal::rational(BigRational::from_integer(
                    (degree as i64).into(),
                )))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::new(coefficients))
    }

    pub fn evaluate(&self, value: &ExactReal) -> Result<ExactReal, ExactRealError> {
        self.coefficients.iter().rev().try_fold(
            ExactReal::rational(BigRational::zero()),
            |result, coefficient| result.try_mul(value)?.try_add(coefficient),
        )
    }

    pub fn try_add(&self, other: &Self) -> Result<Self, ExactRealError> {
        let length = self.coefficients.len().max(other.coefficients.len());
        let coefficients = (0..length)
            .map(|degree| self.coefficient(degree).try_add(&other.coefficient(degree)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::new(coefficients))
    }

    pub fn try_mul(&self, other: &Self) -> Result<Self, ExactRealError> {
        if self.coefficients.is_empty() || other.coefficients.is_empty() {
            return Ok(Self::zero());
        }
        let mut coefficients = vec![
            ExactReal::rational(BigRational::zero());
            self.coefficients.len() + other.coefficients.len() - 1
        ];
        for (left_degree, left) in self.coefficients.iter().enumerate() {
            for (right_degree, right) in other.coefficients.iter().enumerate() {
                let product = left.try_mul(right)?;
                coefficients[left_degree + right_degree] =
                    coefficients[left_degree + right_degree].try_add(&product)?;
            }
        }
        Ok(Self::new(coefficients))
    }
}
