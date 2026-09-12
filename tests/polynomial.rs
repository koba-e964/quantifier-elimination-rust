use num_rational::BigRational;
use quantifier_elimination::Polynomial;
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
