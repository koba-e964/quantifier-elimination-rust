use num_rational::BigRational;
use num_traits::Zero;
use quantifier_elimination::UnivariatePolynomial;

#[test]
fn derivative_and_division_are_exact() {
    let polynomial = UnivariatePolynomial::from_integers(&[-2, 0, 1]);
    assert_eq!(polynomial.derivative().to_string(), "2x");

    let divisor = UnivariatePolynomial::from_integers(&[-1, 1]);
    let (quotient, remainder) = polynomial.div_rem(&divisor);
    assert_eq!(quotient.to_string(), "x + 1");
    assert_eq!(remainder.to_string(), "-1");
}

#[test]
fn sturm_isolates_two_real_roots() {
    let polynomial = UnivariatePolynomial::from_integers(&[-2, 0, 1]);
    let roots = polynomial.isolate_real_roots();
    assert_eq!(roots.len(), 2);
    assert!(roots[0].upper <= roots[1].lower);
    assert!(roots[0].lower < BigRational::zero() && roots[0].upper <= BigRational::zero());
    assert!(roots[1].lower >= BigRational::zero() && roots[1].upper > BigRational::zero());
}

#[test]
fn sturm_handles_no_real_roots() {
    let polynomial = UnivariatePolynomial::from_integers(&[1, 0, 1]);
    assert!(polynomial.isolate_real_roots().is_empty());
}

#[test]
fn refines_an_isolating_interval_exactly() {
    let polynomial = UnivariatePolynomial::from_integers(&[-2, 0, 1]);
    let root = polynomial.isolate_real_roots().remove(1);
    let maximum_width = BigRational::new(1.into(), 100.into());
    let refined = polynomial.refine_root(&root, &maximum_width);

    assert!(refined.width() <= maximum_width);
    assert!(refined.lower < refined.upper);
}
