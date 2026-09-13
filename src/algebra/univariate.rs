use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::fmt;

/// A univariate polynomial with coefficients in `Q`, stored low degree first.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnivariatePolynomial {
    coefficients: Vec<BigRational>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootInterval {
    pub lower: BigRational,
    pub upper: BigRational,
}

impl RootInterval {
    pub fn new(lower: BigRational, upper: BigRational) -> Self {
        assert!(lower < upper, "root intervals must have positive width");
        Self { lower, upper }
    }

    pub fn width(&self) -> BigRational {
        &self.upper - &self.lower
    }
}

impl UnivariatePolynomial {
    pub fn zero() -> Self {
        Self {
            coefficients: Vec::new(),
        }
    }

    pub fn constant(value: BigRational) -> Self {
        if value.is_zero() {
            return Self::zero();
        }
        Self {
            coefficients: vec![value],
        }
    }

    pub fn from_integers(coefficients: &[i64]) -> Self {
        Self::new(
            coefficients
                .iter()
                .map(|value| BigRational::from_integer(BigInt::from(*value)))
                .collect(),
        )
    }

    pub fn new(mut coefficients: Vec<BigRational>) -> Self {
        while coefficients.last().is_some_and(Zero::is_zero) {
            coefficients.pop();
        }
        Self { coefficients }
    }

    pub fn is_zero(&self) -> bool {
        self.coefficients.is_empty()
    }

    pub fn degree(&self) -> Option<usize> {
        if self.is_zero() {
            None
        } else {
            Some(self.coefficients.len() - 1)
        }
    }

    pub fn leading_coefficient(&self) -> Option<&BigRational> {
        self.coefficients.last()
    }

    pub fn coefficient(&self, degree: usize) -> BigRational {
        self.coefficients.get(degree).cloned().unwrap_or_default()
    }

    pub fn evaluate(&self, value: &BigRational) -> BigRational {
        self.coefficients
            .iter()
            .rev()
            .fold(BigRational::zero(), |result, coefficient| {
                result * value + coefficient
            })
    }

    pub fn derivative(&self) -> Self {
        if self.coefficients.len() < 2 {
            return Self::zero();
        }
        Self::new(
            self.coefficients
                .iter()
                .enumerate()
                .skip(1)
                .map(|(degree, coefficient)| coefficient * BigInt::from(degree))
                .collect(),
        )
    }

    pub fn div_rem(&self, divisor: &Self) -> (Self, Self) {
        assert!(!divisor.is_zero(), "division by zero polynomial");
        let divisor_degree = divisor.degree().unwrap();
        let divisor_leading = divisor.leading_coefficient().unwrap();
        let mut remainder = self.clone();
        let mut quotient = vec![
            BigRational::zero();
            self.degree().unwrap_or(0).saturating_sub(divisor_degree) + 1
        ];

        while let Some(remainder_degree) = remainder.degree() {
            if remainder_degree < divisor_degree {
                break;
            }
            let degree = remainder_degree - divisor_degree;
            let coefficient = remainder.leading_coefficient().unwrap() / divisor_leading;
            quotient[degree] = coefficient.clone();
            let mut term = vec![BigRational::zero(); degree];
            term.extend(
                divisor
                    .coefficients
                    .iter()
                    .map(|value| value * &coefficient),
            );
            remainder = remainder - Self::new(term);
        }
        (Self::new(quotient), remainder)
    }

    /// Return the Sturm sequence beginning with this polynomial and its derivative.
    pub fn sturm_sequence(&self) -> Vec<Self> {
        if self.is_zero() {
            return Vec::new();
        }
        let mut sequence = vec![self.clone(), self.derivative()];
        if sequence[1].is_zero() {
            sequence.pop();
            return sequence;
        }
        loop {
            let remainder = sequence[sequence.len() - 2]
                .div_rem(sequence.last().unwrap())
                .1;
            if remainder.is_zero() {
                break;
            }
            sequence.push(-remainder);
        }
        sequence
    }

    /// Isolate every distinct real root using Sturm variation counts.
    pub fn isolate_real_roots(&self) -> Vec<RootInterval> {
        if self.is_zero() || self.degree() == Some(0) {
            return Vec::new();
        }
        let sequence = self.sturm_sequence();
        let bound = self.root_bound();
        let lower = -bound.clone();
        let upper = bound;
        let mut intervals = Vec::new();
        self.split_roots(&sequence, lower, upper, &mut intervals);
        intervals
    }

    pub fn refine_root(
        &self,
        interval: &RootInterval,
        maximum_width: &BigRational,
    ) -> RootInterval {
        assert!(
            !self.is_zero(),
            "cannot refine a root of the zero polynomial"
        );
        assert!(
            interval.lower < interval.upper,
            "root interval must have positive width"
        );
        assert!(
            self.root_count(interval) == 1,
            "interval must isolate one root"
        );
        let sequence = self.sturm_sequence();
        let mut lower = interval.lower.clone();
        let mut upper = interval.upper.clone();
        while &upper - &lower > *maximum_width {
            let midpoint = (&lower + &upper) / BigInt::from(2);
            let left_count = Self::variations_at(&sequence, &lower)
                .saturating_sub(Self::variations_at(&sequence, &midpoint));
            if left_count == 1 {
                upper = midpoint;
            } else {
                lower = midpoint;
            }
        }
        RootInterval::new(lower, upper)
    }

    pub fn count_roots(&self, interval: &RootInterval) -> usize {
        self.root_count(interval)
    }

    pub fn gcd(&self, other: &Self) -> Self {
        let mut left = self.clone();
        let mut right = other.clone();
        while !right.is_zero() {
            let remainder = left.div_rem(&right).1;
            left = right;
            right = remainder;
        }
        if let Some(leading) = left.leading_coefficient().cloned() {
            Self::new(
                left.coefficients
                    .into_iter()
                    .map(|coefficient| coefficient / &leading)
                    .collect(),
            )
        } else {
            Self::zero()
        }
    }

    /// Return the monic polynomial with the same roots as this polynomial.
    pub fn monic(&self) -> Self {
        let Some(leading) = self.leading_coefficient() else {
            return Self::zero();
        };
        Self::new(
            self.coefficients
                .iter()
                .map(|coefficient| coefficient / leading)
                .collect(),
        )
    }

    /// Remove repeated factors and normalize the leading coefficient.
    pub fn square_free_part(&self) -> Self {
        if self.degree().is_none_or(|degree| degree == 0) {
            return self.monic();
        }
        let repeated_factor = self.gcd(&self.derivative());
        self.div_rem(&repeated_factor).0.monic()
    }

    fn root_bound(&self) -> BigRational {
        let leading = self.leading_coefficient().unwrap().abs();
        let max_ratio = self
            .coefficients
            .iter()
            .take(self.coefficients.len() - 1)
            .map(|coefficient| coefficient.abs() / &leading)
            .max()
            .unwrap_or_else(BigRational::zero);
        max_ratio + BigRational::one()
    }

    fn root_count(&self, interval: &RootInterval) -> usize {
        let sequence = self.sturm_sequence();
        Self::variations_at(&sequence, &interval.lower)
            .saturating_sub(Self::variations_at(&sequence, &interval.upper))
    }

    fn variations_at(sequence: &[Self], point: &BigRational) -> usize {
        let signs = sequence
            .iter()
            .map(|polynomial| polynomial.evaluate(point).signum())
            .filter(|sign| !sign.is_zero())
            .collect::<Vec<_>>();
        signs.windows(2).filter(|pair| pair[0] != pair[1]).count()
    }

    fn split_roots(
        &self,
        sequence: &[Self],
        lower: BigRational,
        upper: BigRational,
        intervals: &mut Vec<RootInterval>,
    ) {
        let count = Self::variations_at(sequence, &lower)
            .saturating_sub(Self::variations_at(sequence, &upper));
        if count == 0 {
            return;
        }
        if count == 1 {
            intervals.push(RootInterval::new(lower, upper));
            return;
        }
        let midpoint = (&lower + &upper) / BigInt::from(2);
        if midpoint == lower || midpoint == upper {
            intervals.push(RootInterval::new(lower, upper));
            return;
        }
        self.split_roots(sequence, lower, midpoint.clone(), intervals);
        self.split_roots(sequence, midpoint, upper, intervals);
    }
}

impl std::ops::Add for UnivariatePolynomial {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let length = self.coefficients.len().max(rhs.coefficients.len());
        Self::new(
            (0..length)
                .map(|degree| self.coefficient(degree) + rhs.coefficient(degree))
                .collect(),
        )
    }
}

impl std::ops::Sub for UnivariatePolynomial {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let length = self.coefficients.len().max(rhs.coefficients.len());
        Self::new(
            (0..length)
                .map(|degree| self.coefficient(degree) - rhs.coefficient(degree))
                .collect(),
        )
    }
}

impl std::ops::Neg for UnivariatePolynomial {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(
            self.coefficients
                .into_iter()
                .map(|coefficient| -coefficient)
                .collect(),
        )
    }
}

impl fmt::Display for UnivariatePolynomial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return write!(formatter, "0");
        }
        for (index, coefficient) in self.coefficients.iter().enumerate().rev() {
            if coefficient.is_zero() {
                continue;
            }
            if index != self.degree().unwrap() {
                write!(
                    formatter,
                    "{}",
                    if coefficient.is_negative() {
                        " - "
                    } else {
                        " + "
                    }
                )?;
            } else if coefficient.is_negative() {
                write!(formatter, "-")?;
            }
            let magnitude = coefficient.abs();
            if index == 0 || magnitude != BigRational::one() {
                write!(formatter, "{}", magnitude)?;
            }
            if index > 0 {
                write!(formatter, "x")?;
                if index > 1 {
                    write!(formatter, "^{}", index)?;
                }
            }
        }
        Ok(())
    }
}
