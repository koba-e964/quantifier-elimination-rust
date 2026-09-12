use quantifier_elimination::qe::evaluate::{decide_univariate, eliminate_univariate};
use quantifier_elimination::qe::normalize::to_nnf;
use quantifier_elimination::qe::simplify::simplify;
use quantifier_elimination::{Formula, Polynomial, Relation, RenameError};

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
