use quantifier_elimination::{
    parse_formula, parse_formula_with_names, Formula, Polynomial, Relation,
};

#[test]
fn parses_arithmetic_relations_and_boolean_connectives() {
    let formula = parse_formula("x0^2 - 2 = 0 && !(x1 < 3)").unwrap();
    assert_eq!(
        formula,
        Formula::And(vec![
            Formula::atom(
                Polynomial::variable(0).pow(2) - Polynomial::integer(2),
                Relation::Equal,
            ),
            Formula::Not(Box::new(Formula::atom(
                Polynomial::variable(1) - Polynomial::integer(3),
                Relation::Less,
            ))),
        ])
    );
}

#[test]
fn parses_nested_quantifiers() {
    let formula = parse_formula("exists x1. forall x0. x1 - x0 = 0").unwrap();
    assert_eq!(
        formula,
        Formula::exists(
            1,
            Formula::forall(
                0,
                Formula::atom(
                    Polynomial::variable(1) - Polynomial::variable(0),
                    Relation::Equal
                ),
            ),
        )
    );
}

#[test]
fn reports_location_for_invalid_syntax() {
    let error = parse_formula("x0 =").unwrap_err();
    assert_eq!(error.position, 4);
    assert!(error.message.contains("polynomial term"));
}

#[test]
fn parses_named_free_variables_with_display_names() {
    let parsed = parse_formula_with_names("st + tmp = x").unwrap();

    assert_eq!(
        parsed.formula,
        Formula::atom(
            Polynomial::variable(0) + Polynomial::variable(1) - Polynomial::variable(2),
            Relation::Equal
        )
    );
    assert_eq!(parsed.names.name(0), "st");
    assert_eq!(parsed.names.name(1), "tmp");
    assert_eq!(parsed.names.name(2), "x");
}
