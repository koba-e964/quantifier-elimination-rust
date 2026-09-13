use super::{
    algebraic::AlgebraicReal,
    univariate::{RootInterval, UnivariatePolynomial},
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;
use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactRealError {
    AlgebraicArithmeticNotImplemented,
    AlgebraicRootSampleComparisonUndecidable,
    InvalidAlgebraicRootSample,
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
            (Self::Algebraic(left), Self::Algebraic(right)) => {
                let negated = right.negated();
                if left == &negated {
                    Ok(Self::Rational(BigRational::zero()))
                } else {
                    left.add_algebraic(right)
                        .map(Self::Algebraic)
                        .ok_or(ExactRealError::AlgebraicArithmeticNotImplemented)
                }
            }
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

    pub fn try_mul_rational(&self, value: &BigRational) -> Self {
        match self {
            Self::Rational(left) => Self::Rational(left * value),
            Self::Algebraic(left) => match left.mul_rational(value) {
                Some(result) => Self::Algebraic(result),
                None => Self::Rational(BigRational::zero()),
            },
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgebraicRootSample {
    polynomial: AlgebraicPolynomial,
    interval: RootInterval,
}

impl AlgebraicRootSample {
    pub fn new(polynomial: AlgebraicPolynomial, interval: RootInterval) -> Self {
        Self {
            polynomial,
            interval,
        }
    }

    pub fn polynomial(&self) -> &AlgebraicPolynomial {
        &self.polynomial
    }

    pub fn interval(&self) -> &RootInterval {
        &self.interval
    }

    pub fn refine(&self, maximum_width: &BigRational) -> Result<Self, ExactRealError> {
        if maximum_width <= &BigRational::zero() || self.interval.lower >= self.interval.upper {
            return Err(ExactRealError::InvalidAlgebraicRootSample);
        }
        if self.interval.width() <= *maximum_width {
            return Ok(self.clone());
        }
        let mut intervals = Vec::new();
        isolate_bernstein_to_width(
            &self.polynomial,
            self.polynomial
                .degree()
                .ok_or(ExactRealError::InvalidAlgebraicRootSample)?,
            self.interval.lower.clone(),
            self.interval.upper.clone(),
            maximum_width,
            &mut intervals,
            0,
        )?;
        if intervals.len() != 1 {
            return Err(ExactRealError::InvalidAlgebraicRootSample);
        }
        Ok(Self::new(self.polynomial.clone(), intervals.remove(0)))
    }

    pub fn compare_exact(&self, other: &Self) -> Result<Ordering, ExactRealError> {
        let mut left = self.clone();
        let mut right = other.clone();
        for _ in 0..256 {
            if left.interval.upper < right.interval.lower {
                return Ok(Ordering::Less);
            }
            if right.interval.upper < left.interval.lower {
                return Ok(Ordering::Greater);
            }
            let width = left.interval.width().min(right.interval.width()) / BigInt::from(2);
            left = left.refine(&width)?;
            right = right.refine(&width)?;
        }
        if left.polynomial == right.polynomial {
            return Ok(Ordering::Equal);
        }
        Err(ExactRealError::AlgebraicRootSampleComparisonUndecidable)
    }
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
        if self.coefficients.is_empty() {
            None
        } else {
            Some(self.coefficients.len() - 1)
        }
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

    pub fn linear_root(&self) -> Result<Option<ExactReal>, ExactRealError> {
        if self.degree() != Some(1) {
            return Ok(None);
        }
        let constant = self.coefficient(0).negated();
        let leading = self.coefficient(1);
        let ExactReal::Rational(leading) = leading else {
            return Ok(None);
        };
        if leading.is_zero() {
            return Ok(None);
        }
        Ok(Some(match constant {
            ExactReal::Rational(value) => ExactReal::rational(value / leading),
            ExactReal::Algebraic(value) => ExactReal::algebraic(
                value
                    .mul_rational(&(BigRational::from_integer(1.into()) / leading))
                    .expect("nonzero leading coefficient"),
            ),
        }))
    }

    pub fn isolate_real_roots(&self) -> Result<Vec<RootInterval>, ExactRealError> {
        let Some(degree) = self.degree() else {
            return Ok(Vec::new());
        };
        if degree == 0 {
            return Ok(Vec::new());
        }
        let bound = self.root_bound()?;
        let mut roots = Vec::new();
        isolate_bernstein(self, degree, -bound.clone(), bound, &mut roots, 0)?;
        Ok(roots)
    }

    fn root_bound(&self) -> Result<BigRational, ExactRealError> {
        let degree = self.degree().expect("nonconstant polynomial");
        let leading = abs_exact(&self.coefficient(degree));
        let mut radius = BigRational::from_integer(1.into());
        loop {
            let lhs = leading.try_mul_rational(&radius.pow(degree as i32));
            let mut rhs = ExactReal::rational(BigRational::zero());
            for index in 0..degree {
                let term =
                    abs_exact(&self.coefficient(index)).try_mul_rational(&radius.pow(index as i32));
                rhs = rhs.try_add(&term)?;
            }
            if lhs.compare(&rhs) == Ordering::Greater {
                return Ok(radius + BigRational::from_integer(1.into()));
            }
            radius *= BigInt::from(2);
        }
    }
}

fn abs_exact(value: &ExactReal) -> ExactReal {
    if value.sign() == Ordering::Less {
        value.negated()
    } else {
        value.clone()
    }
}

fn isolate_bernstein(
    polynomial: &AlgebraicPolynomial,
    degree: usize,
    lower: BigRational,
    upper: BigRational,
    roots: &mut Vec<RootInterval>,
    depth: usize,
) -> Result<(), ExactRealError> {
    if depth > 128 || lower == upper {
        return Ok(());
    }
    let coefficients = bernstein_coefficients(polynomial, degree, &lower, &upper)?;
    let variations = sign_variations(&coefficients);
    if variations == 0 {
        return Ok(());
    }
    if variations == 1 {
        roots.push(RootInterval::new(lower, upper));
        return Ok(());
    }
    let midpoint = (&lower + &upper) / BigInt::from(2);
    if midpoint == lower || midpoint == upper {
        roots.push(RootInterval::new(lower, upper));
        return Ok(());
    }
    isolate_bernstein(
        polynomial,
        degree,
        lower,
        midpoint.clone(),
        roots,
        depth + 1,
    )?;
    isolate_bernstein(polynomial, degree, midpoint, upper, roots, depth + 1)
}

fn isolate_bernstein_to_width(
    polynomial: &AlgebraicPolynomial,
    degree: usize,
    lower: BigRational,
    upper: BigRational,
    maximum_width: &BigRational,
    roots: &mut Vec<RootInterval>,
    depth: usize,
) -> Result<(), ExactRealError> {
    if depth > 256 || lower >= upper {
        return Err(ExactRealError::InvalidAlgebraicRootSample);
    }
    let coefficients = bernstein_coefficients(polynomial, degree, &lower, &upper)?;
    let variations = sign_variations(&coefficients);
    if variations == 0 {
        return Ok(());
    }
    let midpoint = (&lower + &upper) / BigInt::from(2);
    if (&upper - &lower) <= *maximum_width {
        roots.push(RootInterval::new(lower, upper));
        return Ok(());
    }
    if midpoint == lower || midpoint == upper {
        return Err(ExactRealError::InvalidAlgebraicRootSample);
    }
    isolate_bernstein_to_width(
        polynomial,
        degree,
        lower,
        midpoint.clone(),
        maximum_width,
        roots,
        depth + 1,
    )?;
    isolate_bernstein_to_width(
        polynomial,
        degree,
        midpoint,
        upper,
        maximum_width,
        roots,
        depth + 1,
    )
}

fn bernstein_coefficients(
    polynomial: &AlgebraicPolynomial,
    degree: usize,
    lower: &BigRational,
    upper: &BigRational,
) -> Result<Vec<ExactReal>, ExactRealError> {
    let affine = AlgebraicPolynomial::new(vec![
        ExactReal::rational(lower.clone()),
        ExactReal::rational(upper - lower),
    ]);
    let mut power = AlgebraicPolynomial::zero();
    for index in (0..=degree).rev() {
        power = power.try_mul(&affine)?;
        power = power.try_add(&AlgebraicPolynomial::new(vec![
            polynomial.coefficient(index)
        ]))?;
    }
    let mut result = Vec::with_capacity(degree + 1);
    for k in 0..=degree {
        let mut coefficient = ExactReal::rational(BigRational::zero());
        for index in 0..=k {
            let factor = BigRational::from_integer(binomial(k, index).into())
                / BigRational::from_integer(binomial(degree, index).into());
            coefficient =
                coefficient.try_add(&power.coefficient(index).try_mul_rational(&factor))?;
        }
        result.push(coefficient);
    }
    Ok(result)
}

fn sign_variations(coefficients: &[ExactReal]) -> usize {
    coefficients
        .iter()
        .map(ExactReal::sign)
        .filter(|sign| *sign != Ordering::Equal)
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|pair| pair[0] != pair[1])
        .count()
}

fn binomial(n: usize, k: usize) -> i64 {
    (0..k).fold(1_i64, |value, index| {
        value * (n - index) as i64 / (index + 1) as i64
    })
}
