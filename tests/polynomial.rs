use num_rational::BigRational;
use quantifier_elimination::{ExactReal, Polynomial, PolynomialEvaluationError, VariableNames};
use std::collections::BTreeMap;

#[test]
fn polynomial_arithmetic_is_exact() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let expression = (x.clone() + Polynomial::integer(1)) * (y - Polynomial::integer(2));
    let mut values = BTreeMap::new();
    values.insert(0, BigRational::from_integer(3.into()));
    values.insert(1, BigRational::from_integer(5.into()));

    assert_eq!(
        expression.evaluate(&values),
        BigRational::from_integer(12.into())
    );
    assert_eq!(expression.to_string(), "-2 - 2*x0 + x0*x1 + x1");
}

#[test]
fn zero_terms_are_removed() {
    let x = Polynomial::variable(0);
    assert!((x.clone() - x).is_zero());
}

#[test]
fn variables_can_have_stable_display_names() {
    let expression = Polynomial::variable(0) * Polynomial::variable(1);
    let names = VariableNames::new().with(0, "x").with(1, "y");

    assert_eq!(expression.to_string_with(&names), "x*y");
    assert_eq!(expression.to_string(), "x0*x1");
}

#[test]
fn evaluates_multivariate_polynomials_at_algebraic_samples() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let expression = x.clone() * x + y;
    let root = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let mut values = BTreeMap::new();
    values.insert(0, root);
    values.insert(1, ExactReal::rational(BigRational::from_integer(3.into())));

    // (sqrt(2))^2 + 3 = 5.
    assert_eq!(
        expression
            .evaluate_exact(&values)
            .unwrap()
            .compare(&ExactReal::rational(BigRational::from_integer(5.into()))),
        std::cmp::Ordering::Equal
    );
}

#[test]
fn reports_missing_algebraic_sample_variables() {
    let expression = Polynomial::variable(2);
    let values = BTreeMap::new();

    // Evaluation is rejected when a polynomial variable has no sample.
    assert_eq!(
        expression.evaluate_exact(&values),
        Err(PolynomialEvaluationError::MissingVariable(2))
    );
}
