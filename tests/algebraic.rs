use num_rational::BigRational;
use std::cmp::Ordering;

use num_traits::Zero;
use quantifier_elimination::{AlgebraicPolynomial, ExactReal, ExactRealError};

#[test]
fn models_polynomials_with_exact_real_coefficients() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer((-2).into())),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);

    assert_eq!(polynomial.degree(), Some(1));
    assert_eq!(polynomial.coefficients().len(), 2);
}

#[test]
fn performs_exact_arithmetic_for_rational_coefficients() {
    let left = ExactReal::rational(BigRational::from_integer(2.into()));
    let right = ExactReal::rational(BigRational::from_integer(3.into()));

    assert_eq!(
        left.try_add(&right),
        Ok(ExactReal::rational(BigRational::from_integer(5.into())))
    );
    assert_eq!(
        left.try_sub(&right),
        Ok(ExactReal::rational(BigRational::from_integer((-1).into())))
    );
    assert_eq!(
        left.try_mul(&right),
        Ok(ExactReal::rational(BigRational::from_integer(6.into())))
    );
    assert_eq!(left.sign(), Ordering::Greater);
}

#[test]
fn reports_unimplemented_algebraic_binary_arithmetic() {
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

    assert_eq!(algebraic.sign(), Ordering::Greater);
    assert_eq!(
        rational.try_add(&algebraic),
        Err(ExactRealError::AlgebraicArithmeticNotImplemented)
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
    assert_eq!(negated.polynomial, root.polynomial);
    assert_eq!(
        negated.interval.lower,
        BigRational::from_integer((-2).into())
    );
    assert_eq!(
        negated.interval.upper,
        BigRational::from_integer((-1).into())
    );
    assert_eq!(ExactReal::algebraic(negated).sign(), Ordering::Less);
}

#[test]
fn normalizes_trailing_zero_coefficients() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer(1.into())),
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::zero()),
    ]);

    assert_eq!(polynomial.degree(), Some(0));
    assert_eq!(polynomial.coefficients().len(), 1);
}
