use num_rational::BigRational;
use quantifier_elimination::algebra::univariate::UnivariatePolynomial;
use quantifier_elimination::cad::lifting::decompose_univariate;
use quantifier_elimination::qe::evaluate::{
    decide_univariate, eliminate, eliminate_one_variable, eliminate_univariate,
};
use quantifier_elimination::qe::normalize::to_nnf;
use quantifier_elimination::qe::simplify::simplify;
use quantifier_elimination::{
    eliminate_with_options, EliminationOptions, Formula, Polynomial, Relation, RenameError,
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
    assert_eq!(
        simplify(&Formula::Or(vec![atom.clone(), atom.clone()])),
        atom
    );
}

#[test]
fn simplifies_quantifier_free_formulas_without_elimination() {
    let formula = Formula::And(vec![
        Formula::atom(Polynomial::zero(), Relation::Equal),
        Formula::atom(Polynomial::variable(0), Relation::Greater),
    ]);

    assert_eq!(
        eliminate(&formula).unwrap(),
        Formula::atom(Polynomial::variable(0), Relation::Greater)
    );
}

#[test]
fn simplifies_atoms_up_to_positive_polynomial_scaling() {
    let scaled = Polynomial::integer(4) * Polynomial::variable(0);
    let canonical = Formula::atom(Polynomial::variable(0), Relation::Less);

    assert_eq!(simplify(&Formula::atom(scaled, Relation::Less)), canonical);
}

#[test]
fn simplifies_a_monomial_zero_equation() {
    let x = Polynomial::variable(0);

    assert_eq!(
        simplify(&Formula::atom(x.pow(3), Relation::Equal)),
        Formula::atom(x, Relation::Equal)
    );
}

#[test]
fn simplifies_the_cubic_symmetric_sign_partition() {
    let k = Polynomial::variable(0);
    let result = simplify(&Formula::Or(vec![
        Formula::And(vec![
            Formula::atom(Polynomial::integer(-1) - k.clone(), Relation::Greater),
            Formula::atom(
                k.clone().pow(3) - Polynomial::integer(3) * k.clone().pow(2),
                Relation::GreaterOrEqual,
            ),
        ]),
        Formula::And(vec![
            Formula::atom(Polynomial::integer(-1) - k.clone(), Relation::Less),
            Formula::atom(
                k.clone().pow(3) - Polynomial::integer(3) * k.clone().pow(2),
                Relation::LessOrEqual,
            ),
        ]),
        Formula::And(vec![
            Formula::atom(Polynomial::integer(-1) - k.clone(), Relation::Equal),
            Formula::atom(k.pow(3), Relation::Equal),
        ]),
    ]));

    assert_eq!(
        result,
        Formula::And(vec![
            Formula::atom(
                Polynomial::integer(1) + Polynomial::variable(0),
                Relation::Greater
            ),
            Formula::atom(
                Polynomial::integer(3) - Polynomial::variable(0),
                Relation::GreaterOrEqual
            ),
        ])
    );
}

#[test]
fn merges_complete_sign_partitions_in_disjunctions() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let x_signs = [Relation::Less, Relation::Equal, Relation::Greater];
    let y_signs = [Relation::Less, Relation::Equal];
    let mut terms = Vec::new();
    for x_relation in x_signs {
        for y_relation in y_signs {
            terms.push(Formula::And(vec![
                Formula::atom(x.clone(), x_relation),
                Formula::atom(y.clone(), y_relation),
            ]));
        }
    }

    assert_eq!(
        simplify(&Formula::Or(terms)),
        Formula::atom(y, Relation::LessOrEqual)
    );
}

#[test]
fn merges_adjacent_sign_relations() {
    let polynomial = Polynomial::variable(0);

    assert_eq!(
        simplify(&Formula::Or(vec![
            Formula::atom(polynomial.clone(), Relation::Less),
            Formula::atom(polynomial.clone(), Relation::Equal),
        ])),
        Formula::atom(polynomial.clone(), Relation::LessOrEqual)
    );
    assert_eq!(
        simplify(&Formula::Or(vec![
            Formula::atom(polynomial.clone(), Relation::Greater),
            Formula::atom(polynomial, Relation::Equal),
        ])),
        Formula::atom(Polynomial::variable(0), Relation::GreaterOrEqual)
    );
}

#[test]
fn simplifies_complementary_sign_bounds_to_equality() {
    let polynomial = Polynomial::variable(0);

    assert_eq!(
        simplify(&Formula::Not(Box::new(Formula::Or(vec![
            Formula::atom(polynomial.clone(), Relation::Less),
            Formula::atom(polynomial.clone(), Relation::Greater),
        ])))),
        Formula::atom(polynomial, Relation::Equal)
    );
}

#[test]
fn recursively_eliminates_vieta_sum_and_product_constraints() {
    let x0 = Polynomial::variable(0);
    let x1 = Polynomial::variable(1);
    let x2 = Polynomial::variable(2);
    let x3 = Polynomial::variable(3);
    let formula = Formula::exists(
        0,
        Formula::exists(
            1,
            Formula::And(vec![
                Formula::atom(x2.clone() - x0.clone() - x1.clone(), Relation::Equal),
                Formula::atom(x3.clone() - x0 * x1, Relation::Equal),
            ]),
        ),
    );
    let eliminated = eliminate(&formula).unwrap();
    assert_eq!(
        eliminated,
        Formula::atom(
            x2.clone() * x2 - Polynomial::integer(4) * x3,
            Relation::GreaterOrEqual
        )
    );

    for ((sum, product), expected) in [((-3, 1), true), ((0, 1), false)] {
        let mut values = BTreeMap::new();
        values.insert(2, BigRational::from_integer(sum.into()));
        values.insert(3, BigRational::from_integer(product.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn special_vieta_rule_matches_general_cad() {
    let x0 = Polynomial::variable(0);
    let x1 = Polynomial::variable(1);
    let x2 = Polynomial::variable(2);
    let x3 = Polynomial::variable(3);
    let formula = Formula::exists(
        0,
        Formula::exists(
            1,
            Formula::And(vec![
                Formula::atom(x2.clone() - x0.clone() - x1.clone(), Relation::Equal),
                Formula::atom(x3 - x0.clone() * x1.clone(), Relation::Equal),
            ]),
        ),
    );

    let with_special_handling = eliminate_with_options(&formula, EliminationOptions::default())
        .unwrap()
        .0;
    let without_special_handling = eliminate_with_options(
        &formula,
        EliminationOptions {
            special_handling: false,
            ..EliminationOptions::default()
        },
    )
    .unwrap()
    .0;

    assert_eq!(with_special_handling, without_special_handling);
}

#[test]
fn automatic_order_matches_explicit_order_at_exact_rational_points() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let t = Polynomial::variable(2);
    let formula = Formula::exists(
        2,
        Formula::atom(y - t.clone() * x - t.clone() * t, Relation::Equal),
    );

    let automatic = eliminate_with_options(&formula, EliminationOptions::default())
        .unwrap()
        .0;
    let explicit = eliminate_with_options(
        &formula,
        EliminationOptions {
            variable_order: Some(vec![0, 1]),
            ..EliminationOptions::default()
        },
    )
    .unwrap()
    .0;
    assert_eq!(automatic, explicit);

    for (x_value, y_value, expected) in [
        (0, 0, true),
        (1, 0, true),
        (0, -1, false),
        (1, -1, false),
        (1, 1, true),
    ] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(x_value.into()));
        values.insert(1, BigRational::from_integer(y_value.into()));
        assert_eq!(automatic.evaluate(&values), Some(expected));
        assert_eq!(explicit.evaluate(&values), Some(expected));
    }
}

#[test]
fn special_vieta_rule_handles_an_implicit_product() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let k = Polynomial::variable(2);
    let formula = Formula::exists(
        0,
        Formula::exists(
            1,
            Formula::And(vec![
                Formula::atom(
                    x.clone().pow(3) + y.clone().pow(3) - Polynomial::integer(3) * x * y,
                    Relation::Equal,
                ),
                Formula::atom(
                    k - Polynomial::variable(0) - Polynomial::variable(1),
                    Relation::Equal,
                ),
            ]),
        ),
    );

    let eliminated = eliminate(&formula).unwrap();
    for (value, expected) in [(-2, false), (-1, false), (0, true), (3, true), (4, false)] {
        let mut values = BTreeMap::new();
        values.insert(2, BigRational::from_integer(value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn special_vieta_rule_accepts_renamed_witnesses_and_reordered_bindings() {
    let left = Polynomial::variable(4);
    let right = Polynomial::variable(7);
    let sum = Polynomial::variable(9);
    let product = Polynomial::variable(11);
    let formula = Formula::exists(
        4,
        Formula::exists(
            7,
            Formula::And(vec![
                Formula::atom(
                    product.clone() - left.clone() * right.clone(),
                    Relation::Equal,
                ),
                Formula::atom(sum.clone() - left - right, Relation::Equal),
            ]),
        ),
    );

    let eliminated = eliminate(&formula).unwrap();

    assert_eq!(
        eliminated,
        Formula::atom(
            sum.clone() * sum - Polynomial::integer(4) * product,
            Relation::GreaterOrEqual
        )
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
    for (value, expected) in [(-1, false), (0, true), (4, true)] {
        let mut assignment = BTreeMap::new();
        assignment.insert(0, BigRational::from_integer(value.into()));
        assert_eq!(eliminated.evaluate(&assignment), Some(expected));
    }
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
    for value in [-3, 0, 5] {
        let mut assignment = BTreeMap::new();
        assignment.insert(0, BigRational::from_integer(value.into()));
        assert_eq!(eliminated.evaluate(&assignment), Some(true));
    }

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
fn recursively_eliminates_nested_quantifiers() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let body = Formula::atom(x.clone() * x + y.clone() * y, Relation::GreaterOrEqual);
    let formula = Formula::exists(0, Formula::forall(1, body));

    assert_eq!(eliminate(&formula).unwrap(), Formula::True);
}

#[test]
fn preserves_shadowed_variable_scopes_during_nested_elimination() {
    let shadowed = Formula::exists(
        0,
        Formula::forall(
            0,
            Formula::atom(
                Polynomial::variable(0) * Polynomial::variable(0),
                Relation::GreaterOrEqual,
            ),
        ),
    );

    assert_eq!(eliminate(&shadowed).unwrap(), Formula::True);
}

#[test]
fn reduces_quantified_boolean_branches_before_the_outer_quantifier() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let body = Formula::And(vec![
        Formula::forall(
            1,
            Formula::atom(
                x.clone() * x.clone() + y.clone() * y,
                Relation::GreaterOrEqual,
            ),
        ),
        Formula::exists(2, Formula::atom(z - x, Relation::Equal)),
    ]);

    assert_eq!(eliminate(&Formula::exists(0, body)).unwrap(), Formula::True);
}

#[test]
fn validates_synthesized_results_on_algebraic_base_cells() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let body = Formula::And(vec![
        Formula::atom(y - x.clone(), Relation::Equal),
        Formula::atom(x.clone() * x - Polynomial::integer(2), Relation::Equal),
    ]);
    let eliminated = eliminate(&Formula::exists(1, body)).unwrap();
    let base_cells = decompose_univariate(&[UnivariatePolynomial::from_integers(&[-2, 0, 1])]);
    let values = base_cells
        .iter()
        .map(|cell| cell.evaluate_formula(&eliminated).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(values, vec![false, true, false, true, false]);
}

#[test]
fn eliminates_boolean_combinations_and_strict_inequalities() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let body = Formula::And(vec![
        Formula::atom(y.clone() * y.clone() - x, Relation::Equal),
        Formula::atom(y, Relation::Greater),
    ]);
    let formula = Formula::exists(1, body);
    let eliminated = eliminate(&formula).unwrap();

    for (value, expected) in [(-1, false), (0, false), (4, true)] {
        let mut assignment = BTreeMap::new();
        assignment.insert(0, BigRational::from_integer(value.into()));
        assert_eq!(eliminated.evaluate(&assignment), Some(expected));
    }
}

#[test]
fn handles_universal_strict_inequalities_and_disequalities() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let universal = Formula::forall(
        1,
        Formula::atom(y.clone() * y.clone() + x.clone(), Relation::Greater),
    );
    let eliminated = eliminate(&universal).unwrap();
    for (value, expected) in [(-1, false), (0, false), (1, true)] {
        let mut assignment = BTreeMap::new();
        assignment.insert(0, BigRational::from_integer(value.into()));
        assert_eq!(eliminated.evaluate(&assignment), Some(expected));
    }

    let existential = Formula::exists(1, Formula::atom(y.clone() * y - x, Relation::NotEqual));
    assert_eq!(eliminate(&existential).unwrap(), Formula::True);
}

#[test]
fn eliminates_existential_quadratic_strict_inequality_with_unbounded_witnesses() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    // For every y, a sufficiently large x makes x^2 + y + 1 positive.
    let formula = Formula::exists(
        0,
        Formula::atom(
            x.clone() * x + y + Polynomial::integer(1),
            Relation::Greater,
        ),
    );

    assert_eq!(eliminate(&formula).unwrap(), Formula::True);
}

#[test]
fn simplifies_universal_quadratic_strict_inequality() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    // x^2 + y > 0 for every x exactly when y > 0.
    let formula = Formula::forall(
        0,
        Formula::atom(x.clone() * x + y.clone(), Relation::Greater),
    );

    assert_eq!(
        eliminate(&formula).unwrap(),
        Formula::atom(y, Relation::Greater)
    );
}

#[test]
fn eliminates_quadratic_lifting_over_an_irrational_base_section() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let body = Formula::atom(
        y.clone() * y - (x.clone() * x + Polynomial::integer(-2)),
        Relation::Equal,
    );
    let formula = Formula::exists(1, body);

    let eliminated = eliminate(&formula).unwrap();
    for (value, expected) in [(0, false), (2, true)] {
        let mut assignment = BTreeMap::new();
        assignment.insert(0, BigRational::from_integer(value.into()));
        assert_eq!(eliminated.evaluate(&assignment), Some(expected));
    }
}

#[test]
fn lifts_linear_polynomials_over_irrational_base_sections() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let body = Formula::And(vec![
        Formula::atom(y - x.clone(), Relation::Equal),
        Formula::atom(x.clone() * x - Polynomial::integer(2), Relation::Equal),
    ]);

    // At x = sqrt(2), the lifted equation y - x has the exact section y = sqrt(2).
    let eliminated = eliminate(&Formula::exists(1, body)).unwrap();
    let mut values = BTreeMap::new();
    values.insert(0, BigRational::from_integer(2.into()));
    assert_eq!(eliminated.evaluate(&values), Some(false));
}

#[test]
fn eliminates_nonlinear_formulas_with_multiple_free_variables() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(1, Formula::atom(y.clone() * y + x + z, Relation::Equal));
    let eliminated = quantifier_elimination::eliminate(&formula).unwrap();
    for (x_value, z_value, expected) in [(0, 0, true), (1, -2, true), (1, 0, false)] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(x_value.into()));
        values.insert(2, BigRational::from_integer(z_value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn synthesizes_multiple_free_conditions_over_an_algebraic_base_section() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(
        2,
        Formula::And(vec![
            Formula::atom(
                x.clone() * x.clone() - Polynomial::integer(2),
                Relation::LessOrEqual,
            ),
            Formula::atom(z.clone() * z + x * y, Relation::Equal),
        ]),
    );
    let eliminated = quantifier_elimination::eliminate(&formula).unwrap();

    for (x_value, y_value, expected) in [(0, 0, true), (2, 0, false), (0, 1, true)] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(x_value.into()));
        values.insert(1, BigRational::from_integer(y_value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn composes_nested_multivariate_cad_elimination_in_variable_order() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::forall(
        2,
        Formula::exists(1, Formula::atom(y.clone() * y + x * z, Relation::Equal)),
    );
    let eliminated = quantifier_elimination::eliminate(&formula).unwrap();

    for (x_value, expected) in [(0, true), (1, false), (-1, false)] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(x_value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn eliminates_vacuous_quantifiers_with_multiple_free_variables() {
    let x = Polynomial::variable(0);
    let z = Polynomial::variable(2);
    let body = Formula::atom(x + z, Relation::Equal);

    assert_eq!(eliminate(&Formula::exists(1, body.clone())).unwrap(), body);
    assert_eq!(eliminate(&Formula::forall(1, body.clone())).unwrap(), body);
}

#[test]
fn simplifies_dead_multivariate_quantifier_branches_before_dispatch() {
    let x = Polynomial::variable(0);
    let z = Polynomial::variable(2);
    let relation = Formula::atom(x + z, Relation::Equal);

    assert_eq!(
        eliminate(&Formula::exists(
            1,
            Formula::And(vec![Formula::False, relation.clone()]),
        ))
        .unwrap(),
        Formula::False
    );
    assert_eq!(
        eliminate(&Formula::forall(
            1,
            Formula::Or(vec![Formula::True, relation]),
        ))
        .unwrap(),
        Formula::True
    );
}

#[test]
fn eliminates_nested_closed_quantifiers_after_recursive_dispatch() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);

    let true_formula = Formula::exists(
        0,
        Formula::forall(
            1,
            Formula::atom(
                y.clone() * y.clone() + x.clone() * x.clone() - Polynomial::integer(1),
                Relation::GreaterOrEqual,
            ),
        ),
    );
    assert_eq!(eliminate(&true_formula).unwrap(), Formula::True);

    let false_formula = Formula::forall(
        0,
        Formula::exists(1, Formula::atom(y.clone() * y - x, Relation::Equal)),
    );
    assert_eq!(eliminate(&false_formula).unwrap(), Formula::False);
}

#[test]
fn eliminates_linear_atomic_formulas_with_multiple_free_variables() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(
        1,
        Formula::atom(
            (x.clone() + z.clone()) * y - Polynomial::integer(1),
            Relation::Equal,
        ),
    );
    let eliminated = eliminate(&formula).unwrap();

    for (x_value, z_value, expected) in [(0, 1, true), (1, -1, false), (2, 3, true)] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(x_value.into()));
        values.insert(2, BigRational::from_integer(z_value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn covers_all_linear_relation_quantifiers_with_multiple_free_variables() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let relations = [
        Relation::Equal,
        Relation::NotEqual,
        Relation::Less,
        Relation::LessOrEqual,
        Relation::Greater,
        Relation::GreaterOrEqual,
    ];

    for relation in relations {
        let polynomial = (x.clone() + z.clone()) * y.clone() + x.clone() - z.clone();
        let existential = eliminate(&Formula::exists(
            1,
            Formula::atom(polynomial.clone(), relation),
        ))
        .unwrap();
        let universal =
            eliminate(&Formula::forall(1, Formula::atom(polynomial, relation))).unwrap();

        let mut nonzero_leading = BTreeMap::new();
        nonzero_leading.insert(0, BigRational::from_integer(1.into()));
        nonzero_leading.insert(2, BigRational::from_integer(0.into()));
        assert_eq!(existential.evaluate(&nonzero_leading), Some(true));
        assert_eq!(universal.evaluate(&nonzero_leading), Some(false));

        let mut zero_leading = BTreeMap::new();
        zero_leading.insert(0, BigRational::from_integer(1.into()));
        zero_leading.insert(2, BigRational::from_integer((-1).into()));
        let expected = match relation {
            Relation::Equal => false,
            Relation::NotEqual => true,
            Relation::Less => false,
            Relation::LessOrEqual => false,
            Relation::Greater => true,
            Relation::GreaterOrEqual => true,
        };
        assert_eq!(existential.evaluate(&zero_leading), Some(expected));
        assert_eq!(universal.evaluate(&zero_leading), Some(expected));
    }
}

#[test]
fn eliminates_negated_linear_atoms_with_multiple_free_variables() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(
        1,
        Formula::Not(Box::new(Formula::atom(
            (x.clone() + z.clone()) * y - Polynomial::integer(1),
            Relation::Less,
        ))),
    );
    let eliminated = eliminate(&formula).unwrap();

    for (x_value, z_value, expected) in [(0, 1, true), (1, -1, false), (2, 3, true)] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(x_value.into()));
        values.insert(2, BigRational::from_integer(z_value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn distributes_supported_boolean_branches_over_multiple_free_variables() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let existential = Formula::exists(
        1,
        Formula::Or(vec![
            Formula::atom(
                (x.clone() + z.clone()) * y.clone() - Polynomial::integer(1),
                Relation::Equal,
            ),
            Formula::atom(y.clone() + x.clone() - z.clone(), Relation::Greater),
        ]),
    );
    let universal = Formula::forall(
        1,
        Formula::And(vec![
            Formula::atom(y.clone() * (x.clone() + z.clone()), Relation::Equal),
            Formula::atom(y - x + z, Relation::Equal),
        ]),
    );

    let existential_result = eliminate(&existential).unwrap();
    let universal_result = eliminate(&universal).unwrap();
    let mut values = BTreeMap::new();
    values.insert(0, BigRational::from_integer(1.into()));
    values.insert(2, BigRational::from_integer(2.into()));
    assert_eq!(existential_result.evaluate(&values), Some(true));
    assert_eq!(universal_result.evaluate(&values), Some(false));
}

#[test]
fn composes_branch_distribution_across_nested_quantifiers() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(
        0,
        Formula::exists(
            1,
            Formula::Or(vec![
                Formula::atom(y.clone() + x.clone() - z.clone(), Relation::Equal),
                Formula::atom(y - x - z, Relation::Greater),
            ]),
        ),
    );

    let eliminated = eliminate(&formula).unwrap();
    let mut values = BTreeMap::new();
    values.insert(2, BigRational::from_integer(3.into()));
    assert_eq!(eliminated.evaluate(&values), Some(true));
}

#[test]
fn factors_variable_independent_guards_from_supported_compound_formulas() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let existential = Formula::exists(
        1,
        Formula::And(vec![
            Formula::atom(x.clone() - z.clone(), Relation::Equal),
            Formula::atom(y.clone() + x.clone(), Relation::Equal),
        ]),
    );
    let universal = Formula::forall(
        1,
        Formula::Or(vec![
            Formula::atom(x.clone() - z.clone(), Relation::Equal),
            Formula::atom(y - x + z, Relation::Equal),
        ]),
    );

    let existential_result = eliminate(&existential).unwrap();
    let universal_result = eliminate(&universal).unwrap();
    let mut values = BTreeMap::new();
    values.insert(0, BigRational::from_integer(1.into()));
    values.insert(2, BigRational::from_integer(1.into()));
    assert_eq!(existential_result.evaluate(&values), Some(true));
    assert_eq!(universal_result.evaluate(&values), Some(true));
    values.insert(2, BigRational::from_integer(2.into()));
    assert_eq!(existential_result.evaluate(&values), Some(false));
    assert_eq!(universal_result.evaluate(&values), Some(false));
}

#[test]
fn substitutes_positive_constant_leading_linear_equalities() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(
        1,
        Formula::And(vec![
            Formula::atom(y.clone() - x.clone() - z.clone(), Relation::Equal),
            Formula::atom(y - x, Relation::Greater),
        ]),
    );
    let eliminated = eliminate(&formula).unwrap();

    for (z_value, expected) in [(-1, false), (0, false), (1, true)] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(2.into()));
        values.insert(2, BigRational::from_integer(z_value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn rejects_universal_conjunctions_with_linear_equalities() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::forall(
        1,
        Formula::And(vec![
            Formula::atom(y - x.clone() - z, Relation::Equal),
            Formula::atom(x, Relation::Greater),
        ]),
    );

    assert_eq!(eliminate(&formula).unwrap(), Formula::False);
}

#[test]
fn substitutes_negative_constant_leading_linear_equalities() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(
        1,
        Formula::And(vec![
            Formula::atom(x.clone() + z.clone() - y.clone(), Relation::Equal),
            Formula::atom(y - x, Relation::Greater),
        ]),
    );
    let eliminated = eliminate(&formula).unwrap();

    for (z_value, expected) in [(-1, false), (0, false), (1, true)] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(2.into()));
        values.insert(2, BigRational::from_integer(z_value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn substitutes_negated_linear_conjunction_branches() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(
        1,
        Formula::And(vec![
            Formula::atom(y.clone() - x.clone(), Relation::Equal),
            Formula::Not(Box::new(Formula::atom(y - z, Relation::LessOrEqual))),
        ]),
    );
    let eliminated = eliminate(&formula).unwrap();

    let mut values = BTreeMap::new();
    values.insert(0, BigRational::from_integer(2.into()));
    values.insert(2, BigRational::from_integer(1.into()));
    assert_eq!(eliminated.evaluate(&values), Some(true));
    values.insert(2, BigRational::from_integer(2.into()));
    assert_eq!(eliminated.evaluate(&values), Some(false));
}

#[test]
fn eliminates_existential_linear_inequality_bounds() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let formula = Formula::exists(
        1,
        Formula::And(vec![
            Formula::atom(y.clone() - x, Relation::Greater),
            Formula::atom(y - z, Relation::LessOrEqual),
        ]),
    );
    let eliminated = eliminate(&formula).unwrap();

    for (x_value, z_value, expected) in [(0, 1, true), (1, 1, false), (2, 1, false)] {
        let mut values = BTreeMap::new();
        values.insert(0, BigRational::from_integer(x_value.into()));
        values.insert(2, BigRational::from_integer(z_value.into()));
        assert_eq!(eliminated.evaluate(&values), Some(expected));
    }
}

#[test]
fn eliminates_universal_linear_conjunctions_and_disjunctions() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let z = Polynomial::variable(2);
    let conjunction = Formula::forall(
        1,
        Formula::And(vec![
            Formula::atom(y.clone() - x.clone(), Relation::Greater),
            Formula::atom(y.clone() - z.clone(), Relation::Less),
        ]),
    );
    let disjunction = Formula::forall(
        1,
        Formula::Or(vec![
            Formula::atom(Polynomial::variable(1) - x, Relation::LessOrEqual),
            Formula::atom(Polynomial::variable(1) - z, Relation::GreaterOrEqual),
        ]),
    );

    assert_eq!(eliminate(&conjunction).unwrap(), Formula::False);
    let eliminated = eliminate(&disjunction).unwrap();
    let mut values = BTreeMap::new();
    values.insert(0, BigRational::from_integer(2.into()));
    values.insert(2, BigRational::from_integer(1.into()));
    assert_eq!(eliminated.evaluate(&values), Some(true));
    values.insert(2, BigRational::from_integer(3.into()));
    assert_eq!(eliminated.evaluate(&values), Some(false));
}
