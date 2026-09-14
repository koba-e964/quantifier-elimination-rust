use num_rational::BigRational;
use std::cmp::Ordering;

use num_traits::{One, Zero};
use quantifier_elimination::{AlgebraicPolynomial, AlgebraicRootSample, ExactReal};

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
fn identifies_rational_roots_inside_wide_intervals() {
    // A linear root need not occur at an interval endpoint or midpoint.
    let polynomial = quantifier_elimination::UnivariatePolynomial::from_integers(&[1, 1]);
    let interval = quantifier_elimination::RootInterval::new(
        BigRational::from_integer((-2).into()),
        BigRational::from_integer(2.into()),
    );
    let algebraic = quantifier_elimination::AlgebraicReal::new(polynomial, interval);

    assert_eq!(
        algebraic.rational_value(),
        Some(BigRational::from_integer((-1).into()))
    );

    // The quadratic roots are -1 and 3; this interval isolates the rational root -1.
    let polynomial = quantifier_elimination::UnivariatePolynomial::from_integers(&[-3, -2, 1]);
    let interval = quantifier_elimination::RootInterval::new(
        BigRational::from_integer((-2).into()),
        BigRational::from_integer(2.into()),
    );
    let algebraic = quantifier_elimination::AlgebraicReal::new(polynomial, interval);

    assert_eq!(
        algebraic.rational_value(),
        Some(BigRational::from_integer((-1).into()))
    );
}

#[test]
fn rational_root_detection_handles_edge_cases() {
    // (x - 1)^2 = 0 has the repeated rational root 1 in [0, 2].
    let repeated = quantifier_elimination::UnivariatePolynomial::from_integers(&[1, -2, 1]);
    let repeated_root = quantifier_elimination::AlgebraicReal::new(
        repeated,
        quantifier_elimination::RootInterval::new(
            BigRational::zero(),
            BigRational::from_integer(2.into()),
        ),
    );
    assert_eq!(
        repeated_root.rational_value(),
        Some(BigRational::from_integer(1.into()))
    );

    // x^2 = 0 has the repeated rational root 0 in [-1, 1].
    let zero_root = quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[0, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer((-1).into()),
            BigRational::from_integer(1.into()),
        ),
    );
    assert_eq!(zero_root.rational_value(), Some(BigRational::zero()));

    // x^2 - 1/4 = 0 has the root 1/2 in [0, 1].
    let half_root = quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::new(vec![
            BigRational::new((-1).into(), 4.into()),
            BigRational::zero(),
            BigRational::one(),
        ]),
        quantifier_elimination::RootInterval::new(BigRational::zero(), BigRational::one()),
    );
    assert_eq!(
        half_root.rational_value(),
        Some(BigRational::new(1.into(), 2.into()))
    );

    // x^2 - 1 = 0 has the root 1 exactly at the lower boundary of [1, 2].
    let boundary_root = quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-1, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::one(),
            BigRational::from_integer(2.into()),
        ),
    );
    assert_eq!(boundary_root.rational_value(), Some(BigRational::one()));

    // x^2 - 2 = 0 has the irrational root sqrt(2) in [1, 2].
    let irrational_root = quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::one(),
            BigRational::from_integer(2.into()),
        ),
    );
    assert_eq!(irrational_root.rational_value(), None);

    // x^2 - 1 = 0 has no root in [2, 3], so no rational value is returned.
    let outside_root = quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-1, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(2.into()),
            BigRational::from_integer(3.into()),
        ),
    );
    assert_eq!(outside_root.rational_value(), None);

    // The nonzero constant 1 has no root, even though [-1, 1] contains zero.
    let constant = quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer((-1).into()),
            BigRational::from_integer(1.into()),
        ),
    );
    assert_eq!(constant.rational_value(), None);
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
fn isolates_roots_with_algebraic_coefficients() {
    let root = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let polynomial = AlgebraicPolynomial::new(vec![
        root.negated(),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);
    let roots = polynomial.isolate_real_roots().unwrap();

    // x - sqrt(2) has one real root between 1 and 2.
    assert_eq!(roots.len(), 1);
    assert!(roots[0].lower < BigRational::from_integer(2.into()));
    assert!(roots[0].upper > BigRational::from_integer(1.into()));
}

#[test]
fn refines_and_compares_algebraic_coefficient_root_samples_exactly() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer((-2).into())),
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);
    let samples = polynomial
        .isolate_real_roots()
        .unwrap()
        .into_iter()
        .map(|interval| AlgebraicRootSample::new(polynomial.clone(), interval))
        .collect::<Vec<_>>();
    assert_eq!(samples.len(), 2);

    let width = BigRational::new(1.into(), 16.into());
    let refined = samples[0].refine(&width).unwrap();
    assert!(refined.interval().width() <= width);
    assert_eq!(samples[0].compare_exact(&refined).unwrap(), Ordering::Equal);
    assert_eq!(
        samples[0].compare_exact(&samples[1]).unwrap(),
        Ordering::Less
    );
}

#[test]
fn evaluates_signs_at_rational_defining_polynomial_root_samples_exactly() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer((-2).into())),
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);
    let sample = AlgebraicRootSample::new(
        polynomial.clone(),
        polynomial.isolate_real_roots().unwrap().pop().unwrap(),
    );

    assert_eq!(sample.sign_of(&polynomial), Ok(Ordering::Equal));
}

#[test]
fn compares_common_roots_from_different_defining_polynomials() {
    let left_polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer((-2).into())),
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);
    let right_polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer((-4).into())),
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::from_integer(2.into())),
    ]);
    let left = AlgebraicRootSample::new(
        left_polynomial.clone(),
        left_polynomial.isolate_real_roots().unwrap().pop().unwrap(),
    );
    let right = AlgebraicRootSample::new(
        right_polynomial.clone(),
        right_polynomial
            .isolate_real_roots()
            .unwrap()
            .pop()
            .unwrap(),
    );

    assert_eq!(left.compare_exact(&right), Ok(Ordering::Equal));
}

#[test]
fn refines_an_algebraic_root_sample_at_an_interval_endpoint() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);
    let sample = AlgebraicRootSample::new(
        polynomial.clone(),
        quantifier_elimination::RootInterval::new(
            BigRational::zero(),
            BigRational::new(1.into(), 2.into()),
        ),
    );

    let refined = sample
        .refine(&BigRational::new(1.into(), 16.into()))
        .unwrap();
    assert!(refined.interval().width() <= BigRational::new(1.into(), 16.into()));
    assert_eq!(refined.sign_of(&polynomial), Ok(Ordering::Equal));
}

#[test]
fn isolates_a_repeated_algebraic_coefficient_root() {
    let root = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let twice_negative_root = root
        .try_mul(&ExactReal::rational(BigRational::from_integer((-2).into())))
        .unwrap();
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer(2.into())),
        twice_negative_root,
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);

    let roots = polynomial.isolate_real_roots().unwrap();
    assert_eq!(roots.len(), 1);
}

#[test]
fn isolates_roots_of_a_quadratic_with_an_algebraic_constant() {
    let square_root_two = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let polynomial = AlgebraicPolynomial::new(vec![
        square_root_two.negated(),
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);

    assert_eq!(polynomial.isolate_real_roots().unwrap().len(), 2);
}

#[test]
fn rejects_a_quadratic_with_a_positive_algebraic_constant() {
    let square_root_two = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let polynomial = AlgebraicPolynomial::new(vec![
        square_root_two,
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);

    assert!(polynomial.isolate_real_roots().unwrap().is_empty());
}

#[test]
fn isolates_distinct_repeated_rational_roots_exactly() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer(4.into())),
        ExactReal::rational(BigRational::from_integer((-4).into())),
        ExactReal::rational(BigRational::from_integer((-3).into())),
        ExactReal::rational(BigRational::from_integer(2.into())),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);
    let roots = polynomial.isolate_real_roots().unwrap();

    assert_eq!(roots.len(), 2);
    assert!(roots.windows(2).all(|pair| pair[0].upper <= pair[1].lower));
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

#[test]
fn carries_algebraic_coefficient_root_samples_as_exact_values() {
    let polynomial = AlgebraicPolynomial::new(vec![
        ExactReal::rational(BigRational::from_integer((-2).into())),
        ExactReal::rational(BigRational::zero()),
        ExactReal::rational(BigRational::from_integer(1.into())),
    ]);
    let mut roots = polynomial.isolate_real_roots().unwrap();
    roots.sort_by(|left, right| left.lower.cmp(&right.lower));
    let sample = AlgebraicRootSample::new(polynomial.clone(), roots.pop().unwrap());
    let root = ExactReal::algebraic_root(sample);
    let one = ExactReal::rational(BigRational::from_integer(1.into()));
    let two = ExactReal::rational(BigRational::from_integer(2.into()));

    assert_eq!(root.compare(&one), Ordering::Greater);
    assert_eq!(root.try_add(&one).unwrap().compare(&two), Ordering::Greater);
    assert_eq!(root.try_mul(&two).unwrap().compare(&two), Ordering::Greater);
}

#[test]
fn normalizes_algebraic_definitions_and_collapses_rational_roots() {
    let repeated_definition =
        quantifier_elimination::UnivariatePolynomial::from_integers(&[4, -8, 4]);
    let root = quantifier_elimination::AlgebraicReal::new(
        repeated_definition,
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(0.into()),
            BigRational::from_integer(2.into()),
        ),
    );

    assert_eq!(root.polynomial.square_free_part().to_string(), "x - 1");
    assert_eq!(
        ExactReal::algebraic(root),
        ExactReal::rational(BigRational::from_integer(1.into()))
    );
}

#[test]
fn cancels_semantically_equal_algebraic_values_with_different_intervals() {
    let polynomial = quantifier_elimination::UnivariatePolynomial::from_integers(&[-2, 0, 1]);
    let left = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        polynomial.clone(),
        quantifier_elimination::RootInterval::new(
            BigRational::from_integer(1.into()),
            BigRational::from_integer(2.into()),
        ),
    ));
    let right = ExactReal::algebraic(quantifier_elimination::AlgebraicReal::new(
        polynomial,
        quantifier_elimination::RootInterval::new(
            BigRational::new(7.into(), 5.into()),
            BigRational::new(3.into(), 2.into()),
        ),
    ));

    assert_eq!(left.try_sub(&right).unwrap().sign(), Ordering::Equal);
}
