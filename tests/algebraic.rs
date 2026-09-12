use num_rational::BigRational;
use quantifier_elimination::{AlgebraicPolynomial, ExactReal};

#[test]
fn models_polynomials_with_exact_real_coefficients() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer((-2).into())),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);

    assert_eq!(polynomial.degree(), Some(1));
    assert_eq!(polynomial.coefficients().len(), 2);
}
