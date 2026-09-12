use quantifier_elimination::qe::evaluate::decide_univariate;
use quantifier_elimination::{Formula, Polynomial, Relation};

#[test]
fn decides_existential_polynomial_formulas() {
    let x = Polynomial::variable(0);
    let square_minus_two = x.clone() * x.clone() + Polynomial::integer(-2);
    let square_plus_one = x.clone() * x + Polynomial::integer(1);

    assert!(decide_univariate(&Formula::exists(
        0,
        Formula::atom(square_minus_two, Relation::Equal),
    ))
    .unwrap());
    assert!(!decide_univariate(&Formula::exists(
        0,
        Formula::atom(square_plus_one, Relation::Equal),
    ))
    .unwrap());
}

#[test]
fn decides_a_universal_polynomial_formula() {
    let x = Polynomial::variable(0);
    let square = x.clone() * x;
    assert!(decide_univariate(&Formula::forall(
        0,
        Formula::atom(square, Relation::GreaterOrEqual),
    ))
    .unwrap());
}
