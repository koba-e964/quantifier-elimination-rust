use super::univariate::{RootInterval, UnivariatePolynomial};
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
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

    pub fn add_algebraic(&self, other: &Self) -> Option<Self> {
        let polynomial = resultant_for_sum(&self.polynomial, &other.polynomial)?;
        select_operation_root(&polynomial, self, other, Operation::Add)
    }

    pub fn mul_algebraic(&self, other: &Self) -> Option<Self> {
        let polynomial = resultant_for_product(&self.polynomial, &other.polynomial)?;
        select_operation_root(&polynomial, self, other, Operation::Mul)
    }

    pub fn rational_value(&self) -> Option<num_rational::BigRational> {
        if let Some(root) = rational_root_in_interval(&self.polynomial, &self.interval) {
            return Some(root);
        }
        let midpoint = (&self.interval.lower + &self.interval.upper) / num_bigint::BigInt::from(2);
        for value in [&self.interval.lower, &self.interval.upper, &midpoint] {
            if self.polynomial.evaluate(value).is_zero() {
                return Some(value.clone());
            }
        }
        None
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
        if let (Some(left), Some(right)) = (self.rational_value(), other.rational_value()) {
            return left.cmp(&right);
        }
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

fn rational_root_in_interval(
    polynomial: &UnivariatePolynomial,
    interval: &RootInterval,
) -> Option<BigRational> {
    let degree = polynomial.degree()?;
    if degree == 0 {
        return None;
    }
    let scale = (0..=degree).fold(BigInt::one(), |scale, degree| {
        scale.lcm(polynomial.coefficient(degree).denom())
    });
    let integer_coefficients = (0..=degree)
        .map(|degree| {
            polynomial.coefficient(degree).numer()
                * (&scale / polynomial.coefficient(degree).denom())
        })
        .collect::<Vec<_>>();
    let constant = integer_coefficients[0].abs();
    let leading = integer_coefficients[degree].abs();
    if leading.is_zero() {
        return None;
    }
    if constant.is_zero() {
        let root = BigRational::zero();
        return (interval.lower <= root && root <= interval.upper).then_some(root);
    }
    for numerator in positive_divisors(&constant) {
        for denominator in positive_divisors(&leading) {
            for signed_numerator in [numerator.clone(), -numerator.clone()] {
                let root = BigRational::new(signed_numerator, denominator.clone());
                if interval.lower <= root
                    && root <= interval.upper
                    && polynomial.evaluate(&root).is_zero()
                {
                    return Some(root);
                }
            }
        }
    }
    None
}

fn positive_divisors(value: &BigInt) -> Vec<BigInt> {
    // TODO: replace trial division with an efficient integer factorization/divisor generator.
    let mut divisors = Vec::new();
    let mut candidate = BigInt::one();
    while &candidate * &candidate <= *value {
        let (quotient, remainder) = value.div_rem(&candidate);
        if remainder.is_zero() {
            divisors.push(candidate.clone());
            if quotient != candidate {
                divisors.push(quotient);
            }
        }
        candidate += BigInt::one();
    }
    divisors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_rational_roots_inside_intervals() {
        // x^2 - 2x - 3 = 0 has the root -1 in [-2, 2].
        let polynomial = UnivariatePolynomial::from_integers(&[-3, -2, 1]);
        let interval = RootInterval::new(
            BigRational::from_integer((-2).into()),
            BigRational::from_integer(2.into()),
        );

        assert_eq!(
            rational_root_in_interval(&polynomial, &interval),
            Some(BigRational::from_integer((-1).into()))
        );
    }

    #[test]
    fn finds_degree_three_rational_roots() {
        // x^3 - 6x^2 + 11x - 6 = 0 has the root 2 in [3/2, 5/2].
        let integer_root = UnivariatePolynomial::from_integers(&[-6, 11, -6, 1]);
        let integer_interval = RootInterval::new(
            BigRational::new(3.into(), 2.into()),
            BigRational::new(5.into(), 2.into()),
        );
        assert_eq!(
            rational_root_in_interval(&integer_root, &integer_interval),
            Some(BigRational::from_integer(2.into()))
        );

        // x^3 - 1/8 = 0 has the non-integral rational root 1/2 in [0, 1].
        let non_integral_root = UnivariatePolynomial::new(vec![
            BigRational::new((-1).into(), 8.into()),
            BigRational::zero(),
            BigRational::zero(),
            BigRational::one(),
        ]);
        let positive_interval = RootInterval::new(BigRational::zero(), BigRational::one());
        assert_eq!(
            rational_root_in_interval(&non_integral_root, &positive_interval),
            Some(BigRational::new(1.into(), 2.into()))
        );

        // x^3 - 2 = 0 has no rational root in [1, 2].
        let irrational_root = UnivariatePolynomial::from_integers(&[-2, 0, 0, 1]);
        assert_eq!(
            rational_root_in_interval(&irrational_root, &positive_interval),
            None
        );
    }

    #[test]
    fn handles_rational_root_edge_cases() {
        // x^2 = 0 has the root 0 in [-1, 1].
        let zero = UnivariatePolynomial::from_integers(&[0, 0, 1]);
        let wide_interval = RootInterval::new(
            BigRational::from_integer((-1).into()),
            BigRational::from_integer(1.into()),
        );
        assert_eq!(
            rational_root_in_interval(&zero, &wide_interval),
            Some(BigRational::zero())
        );

        // x^2 - 1/4 = 0 has the root 1/2 in [0, 1].
        let half = UnivariatePolynomial::new(vec![
            BigRational::new((-1).into(), 4.into()),
            BigRational::zero(),
            BigRational::one(),
        ]);
        let positive_interval = RootInterval::new(BigRational::zero(), BigRational::one());
        assert_eq!(
            rational_root_in_interval(&half, &positive_interval),
            Some(BigRational::new(1.into(), 2.into()))
        );

        // x^2 - 2 = 0 has no rational root in [1, 2].
        let irrational = UnivariatePolynomial::from_integers(&[-2, 0, 1]);
        assert_eq!(
            rational_root_in_interval(&irrational, &positive_interval),
            None
        );

        // The nonzero constant 1 has no root in [-1, 1].
        let constant = UnivariatePolynomial::from_integers(&[1]);
        assert_eq!(rational_root_in_interval(&constant, &wide_interval), None);
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

fn determinant(matrix: &[Vec<UnivariatePolynomial>]) -> UnivariatePolynomial {
    if matrix.is_empty() {
        return UnivariatePolynomial::constant(BigRational::from_integer(1.into()));
    }
    if matrix.len() == 1 {
        return matrix[0][0].clone();
    }
    let mut result = UnivariatePolynomial::zero();
    for column in 0..matrix.len() {
        let minor = matrix
            .iter()
            .skip(1)
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter_map(|(index, value)| (index != column).then_some(value.clone()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let term = multiply(&matrix[0][column], &determinant(&minor));
        result = if column % 2 == 0 {
            result + term
        } else {
            result + -term
        };
    }
    result
}

fn resultant_for_sum(
    left: &UnivariatePolynomial,
    right: &UnivariatePolynomial,
) -> Option<UnivariatePolynomial> {
    let left_degree = left.degree()?;
    let right_degree = right.degree()?;
    let right_substituted = substitute_sum(right);
    resultant_matrix(left, &right_substituted, left_degree, right_degree)
}

fn resultant_for_product(
    left: &UnivariatePolynomial,
    right: &UnivariatePolynomial,
) -> Option<UnivariatePolynomial> {
    let left_degree = left.degree()?;
    let right_degree = right.degree()?;
    let right_homogeneous = substitute_product(right, right_degree);
    resultant_matrix(left, &right_homogeneous, left_degree, right_degree)
}

fn substitute_sum(polynomial: &UnivariatePolynomial) -> Vec<UnivariatePolynomial> {
    let degree = polynomial.degree().unwrap_or(0);
    let mut result = vec![UnivariatePolynomial::zero(); degree + 1];
    for source_degree in 0..=degree {
        let coefficient = polynomial.coefficient(source_degree);
        for z_degree in 0..=source_degree {
            let y_degree = source_degree - z_degree;
            let binomial = binomial(source_degree, z_degree);
            let sign = if y_degree % 2 == 0 { 1 } else { -1 };
            let value = coefficient.clone() * BigRational::from_integer((sign * binomial).into());
            let mut coefficients = vec![BigRational::zero(); z_degree];
            coefficients.push(value);
            result[y_degree] = result[y_degree].clone() + UnivariatePolynomial::new(coefficients);
        }
    }
    result
}

fn substitute_product(
    polynomial: &UnivariatePolynomial,
    degree: usize,
) -> Vec<UnivariatePolynomial> {
    let mut result = vec![UnivariatePolynomial::zero(); degree + 1];
    for z_degree in 0..=degree {
        let coefficient = polynomial.coefficient(z_degree);
        let mut coefficients = vec![BigRational::zero(); z_degree];
        coefficients.push(coefficient);
        result[degree - z_degree] =
            result[degree - z_degree].clone() + UnivariatePolynomial::new(coefficients);
    }
    result
}

fn resultant_matrix(
    left: &UnivariatePolynomial,
    right: &[UnivariatePolynomial],
    left_degree: usize,
    right_degree: usize,
) -> Option<UnivariatePolynomial> {
    let size = left_degree + right_degree;
    if size == 0 {
        return None;
    }
    let mut matrix = vec![vec![UnivariatePolynomial::zero(); size]; size];
    for row in 0..right_degree {
        for degree in 0..=left_degree {
            matrix[row][row + degree] = UnivariatePolynomial::constant(left.coefficient(degree));
        }
    }
    for row in 0..left_degree {
        matrix[right_degree + row][row..(right_degree + row + 1)]
            .clone_from_slice(&right[..(right_degree + 1)]);
    }
    Some(determinant(&matrix))
}

#[derive(Clone, Copy)]
enum Operation {
    Add,
    Mul,
}

fn select_operation_root(
    polynomial: &UnivariatePolynomial,
    left: &AlgebraicReal,
    right: &AlgebraicReal,
    operation: Operation,
) -> Option<AlgebraicReal> {
    let square_free = polynomial
        .div_rem(&polynomial.gcd(&polynomial.derivative()))
        .0;
    let roots = square_free.isolate_real_roots();
    let mut left = left.clone();
    let mut right = right.clone();
    for _ in 0..64 {
        let interval = match operation {
            Operation::Add => RootInterval::new(
                &left.interval.lower + &right.interval.lower,
                &left.interval.upper + &right.interval.upper,
            ),
            Operation::Mul => operation_interval(&left, &right),
        };
        let candidates = roots
            .iter()
            .filter(|root| root.upper >= interval.lower && root.lower <= interval.upper)
            .cloned()
            .collect::<Vec<_>>();
        if candidates.len() == 1 {
            return Some(AlgebraicReal::new(
                square_free.clone(),
                candidates[0].clone(),
            ));
        }
        let width = left.interval.width().min(right.interval.width()) / BigInt::from(2);
        left = left.refine(&width);
        right = right.refine(&width);
    }
    None
}

fn operation_interval(left: &AlgebraicReal, right: &AlgebraicReal) -> RootInterval {
    let values = [
        &left.interval.lower * &right.interval.lower,
        &left.interval.lower * &right.interval.upper,
        &left.interval.upper * &right.interval.lower,
        &left.interval.upper * &right.interval.upper,
    ];
    let lower = values.iter().min().unwrap().clone();
    let upper = values.iter().max().unwrap().clone();
    if lower == upper {
        RootInterval::new(lower.clone(), upper + BigRational::from_integer(1.into()))
    } else {
        RootInterval::new(lower, upper)
    }
}

fn binomial(n: usize, k: usize) -> i64 {
    (0..k).fold(1_i64, |value, index| {
        value * (n - index) as i64 / (index + 1) as i64
    })
}
