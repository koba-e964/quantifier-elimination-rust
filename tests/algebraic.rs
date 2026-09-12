use num_rational::BigRational;
use std::cmp::Ordering;

use num_traits::Zero;
use quantifier_elimination::{AlgebraicPolynomial, ExactReal};

#[test]
fn models_polynomials_with_exact_real_coefficients() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer((-2).into())),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);

    // The coefficient list [-2, 1] represents the linear polynomial x - 2.
    assert_eq!(polynomial.degree(), Some(1));
    // A degree-one polynomial has two stored coefficients, including its constant term.
    assert_eq!(polynomial.coefficients().len(), 2);
}

#[test]
fn performs_exact_arithmetic_for_rational_coefficients() {
    let left = ExactReal::rational(BigRational::from_integer(2.into()));
    let right = ExactReal::rational(BigRational::from_integer(3.into()));

    // 2 + 3 = 5 exactly.
    assert_eq!(
        left.try_add(&right),
        Ok(ExactReal::rational(BigRational::from_integer(5.into())))
    );
    // 2 - 3 = -1 exactly.
    assert_eq!(
        left.try_sub(&right),
        Ok(ExactReal::rational(BigRational::from_integer((-1).into())))
    );
    // 2 * 3 = 6 exactly.
    assert_eq!(
        left.try_mul(&right),
        Ok(ExactReal::rational(BigRational::from_integer(6.into())))
    );
    // 2 is positive.
    assert_eq!(left.sign(), Ordering::Greater);
}

#[test]
fn compares_mixed_values_exactly() {
    let rational = ExactReal::rational(BigRational::from_integer(2.into()));
    let algebraic = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::new(vec![
            BigRational::from_integer((-2).into()),
            BigRational::zero(),
            BigRational::from_integer(1.into()),
        ]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));

    // The selected root of x^2 - 2 in (1, 2) is sqrt(2), which is positive.
    assert_eq!(algebraic.sign(), Ordering::Greater);
    // 2 + sqrt(2) > sqrt(2).
    // 2 negates to -2 exactly.
    assert_eq!(
        rational.try_add(&algebraic).unwrap().compare(&algebraic),
        Ordering::Greater
    );
}

#[test]
fn negates_rational_and_algebraic_values_exactly() {
    let rational = ExactReal::rational(BigRational::from_integer(2.into()));
    assert_eq!(
        rational.negated(),
        ExactReal::rational(BigRational::from_integer((-2).into()))
    );

    let root = quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    );
    let negated = root.negated();
    // x^2 - 2 is unchanged by x -> -x.
    assert_eq!(negated.polynomial, root.polynomial);
    // The isolating interval (1, 2) becomes (-2, -1).
    assert_eq!(
        negated.interval.lower,
        BigRational::from_integer((-2).into())
    );
    assert_eq!(
        negated.interval.upper,
        BigRational::from_integer((-1).into())
    );
    // The negated root is -sqrt(2), which is negative.
    assert_eq!(ExactReal::algebraic(negated).sign(), Ordering::Less);
}

#[test]
fn normalizes_trailing_zero_coefficients() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer(1.into())),
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::zero()),
    ]);

    // Trailing zero coefficients do not change the polynomial's degree.
    assert_eq!(polynomial.degree(), Some(0));
    // [1, 0, 0] is normalized to the single coefficient [1].
    assert_eq!(polynomial.coefficients().len(), 1);
}

#[test]
fn evaluates_and_differentiates_algebraic_coefficient_polynomials() {
    let root = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let polynomial = AlgebraicPolynomial::new(vec![
        root.clone(),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);

    // The polynomial sqrt(2) + x evaluates to sqrt(2) + 1 at x = 1.
    assert_eq!(
        polynomial
            .evaluate(&ExactReal::rational(BigRational::from_integer(1.into())))
            .unwrap()
            .compare(
                &root
                    .try_add(&ExactReal::rational(BigRational::from_integer(1.into())))
                    .unwrap()
            ),
        Ordering::Equal
    );
    // The derivative of sqrt(2) + x is the constant polynomial 1.
    assert_eq!(polynomial.derivative().unwrap().degree(), Some(0));
    assert_eq!(
        polynomial.derivative().unwrap().coefficient(0).sign(),
        Ordering::Greater
    );
}

#[test]
fn performs_exact_mixed_rational_algebraic_arithmetic() {
    let root = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let one = ExactReal::rational(BigRational::from_integer(1.into()));
    let three = ExactReal::rational(BigRational::from_integer(3.into()));

    // sqrt(2) + 1 < 3.
    assert_eq!(root.try_add(&one).unwrap().compare(&three), Ordering::Less);
    // sqrt(2) - 1 > 0.
    assert_eq!(root.try_sub(&one).unwrap().sign(), Ordering::Greater);
    // sqrt(2) * 1 = sqrt(2).
    assert_eq!(root.try_mul(&one).unwrap().compare(&root), Ordering::Equal);
    // sqrt(2) > 1.
    assert_eq!(root.compare(&one), Ordering::Greater);
}

#[test]
fn performs_exact_algebraic_algebraic_arithmetic() {
    let root = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let two = ExactReal::rational(BigRational::from_integer(2.into()));

    // sqrt(2) + sqrt(2) = 2 * sqrt(2) > 2.
    assert_eq!(
        root.try_add(&root).unwrap().compare(&two),
        Ordering::Greater
    );
    // sqrt(2) - sqrt(2) = 0.
    assert_eq!(root.try_sub(&root).unwrap().sign(), Ordering::Equal);
    // sqrt(2) * sqrt(2) = 2.
    assert_eq!(root.try_mul(&root).unwrap().compare(&two), Ordering::Equal);
}
