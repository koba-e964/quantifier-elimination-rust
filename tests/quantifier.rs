use num_rational::BigRational;
use quantifier_elimination::algebra::univariate::UnivariatePolynomial;
use quantifier_elimination::cad::lifting::decompose_univariate;
use quantifier_elimination::qe::evaluate::{
    decide_univariate, eliminate, eliminate_one_variable, eliminate_univariate,
};
use quantifier_elimination::qe::normalize::to_nnf;
use quantifier_elimination::qe::simplify::simplify;
use quantifier_elimination::{
    Formula, Polynomial, QuantifierEvaluationError, Relation, RenameError,
};
use std::collections::BTreeMap;

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

#[test]
fn eliminates_a_closed_formula_to_a_quantifier_free_constant() {
    let x = Polynomial::variable(0);
    let formula = Formula::exists(
        0,
        Formula::atom(x.clone() * x + Polynomial::integer(1), Relation::Equal),
    );

    assert_eq!(eliminate_univariate(&formula).unwrap(), Formula::False);
}

#[test]
fn normalizes_negation_and_dualizes_quantifiers() {
    let x = Polynomial::variable(0);
    let formula = Formula::Not(Box::new(Formula::forall(
        0,
        Formula::atom(x, Relation::Less),
    )));
    let expected = Formula::exists(
        0,
        Formula::atom(Polynomial::variable(0), Relation::GreaterOrEqual),
    );

    assert_eq!(to_nnf(&formula), expected);
}

#[test]
fn simplifies_boolean_identities_and_flattens_connectives() {
    let x = Polynomial::variable(0);
    let atom = Formula::atom(x, Relation::Equal);
    let formula = Formula::And(vec![
        Formula::True,
        Formula::And(vec![atom.clone(), Formula::True]),
    ]);
    assert_eq!(simplify(&formula), atom);
    assert_eq!(
        simplify(&Formula::Or(vec![Formula::False, Formula::True])),
        Formula::True
    );
}

#[test]
fn tracks_free_variables_across_quantifier_scopes() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let formula = Formula::exists(
        0,
        Formula::And(vec![
            Formula::atom(x + y.clone(), Relation::Equal),
            Formula::atom(y, Relation::Greater),
        ]),
    );

    assert_eq!(
        formula.free_variables().into_iter().collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn alpha_renames_without_capturing_free_variables() {
    let x = Polynomial::variable(0);
    let formula = Formula::exists(0, Formula::atom(x, Relation::Equal));
    let renamed = formula.alpha_rename(0, 2).unwrap();

    assert_eq!(renamed.free_variables().len(), 0);
    assert!(matches!(renamed, Formula::Quantified { variable: 2, .. }));

    let y = Polynomial::variable(1);
    let capturing = Formula::exists(0, Formula::atom(y, Relation::Equal));
    assert_eq!(capturing.alpha_rename(0, 1), Err(RenameError::WouldCapture));
}

#[test]
fn eliminates_one_variable_and_preserves_the_free_variable() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let body = Formula::atom(y.clone() * y - x.clone(), Relation::Equal);
    let formula = Formula::exists(1, body);
    let eliminated = eliminate_one_variable(&formula, 0, 1).unwrap();

    assert_eq!(
        eliminated.free_variables().into_iter().collect::<Vec<_>>(),
        vec![0]
    );
    assert!(eliminated.is_quantifier_free());
    let cells = decompose_univariate(&[UnivariatePolynomial::from_integers(&[0, 1])]);
    let values = cells
        .iter()
        .map(|cell| cell.evaluate_formula(&eliminated).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values, vec![false, true, true]);
}

#[test]
fn eliminates_a_universal_variable() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let body = Formula::atom(y.clone() * y + x.clone() * x, Relation::GreaterOrEqual);
    let formula = Formula::forall(1, body);
    let eliminated = eliminate_one_variable(&formula, 0, 1).unwrap();
    assert_eq!(eliminated, Formula::True);

    let cells = decompose_univariate(&[UnivariatePolynomial::from_integers(&[0, 0, 1])]);
    assert!(cells
        .iter()
        .all(|cell| cell.evaluate_formula(&eliminated).unwrap()));
}

#[test]
fn evaluates_quantifier_free_formulas_at_exact_rational_points() {
    let x = Polynomial::variable(0);
    let formula = Formula::atom(x, Relation::GreaterOrEqual);
    let mut values = BTreeMap::new();
    values.insert(0, BigRational::from_integer((-1).into()));
    assert_eq!(formula.evaluate(&values), Some(false));
    values.insert(0, BigRational::from_integer(2.into()));
    assert_eq!(formula.evaluate(&values), Some(true));
    assert_eq!(Formula::exists(0, Formula::True).evaluate(&values), None);
    assert_eq!(
        Formula::atom(Polynomial::variable(1), Relation::Equal).evaluate(&values),
        None
    );
}

#[test]
fn dispatches_supported_elimination_paths() {
    let x = Polynomial::variable(0);
    let closed = Formula::exists(0, Formula::atom(x.clone() * x.clone(), Relation::Equal));
    assert_eq!(
        quantifier_elimination::eliminate(&closed).unwrap(),
        Formula::True
    );

    let y = Polynomial::variable(1);
    let parameterized = Formula::exists(1, Formula::atom(y.clone() * y - x, Relation::Equal));
    assert!(eliminate(&parameterized).unwrap().is_quantifier_free());
}

#[test]
fn reports_unsupported_elimination_shapes() {
    let x = Polynomial::variable(0);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(1, Formula::atom(x + z, Relation::Equal));
    assert_eq!(
        quantifier_elimination::eliminate(&formula),
        Err(QuantifierEvaluationError::WrongVariable)
    );
    assert_eq!(
        quantifier_elimination::eliminate(&Formula::True),
        Err(QuantifierEvaluationError::WrongVariable)
    );
}
