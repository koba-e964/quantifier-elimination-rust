use super::algebraic::AlgebraicReal;
use num_rational::BigRational;

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
