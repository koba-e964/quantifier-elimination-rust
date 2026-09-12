use num_rational::BigRational;
use num_traits::Zero;
use quantifier_elimination::algebra::univariate::UnivariatePolynomial;
use quantifier_elimination::cad::lifting::{
    cell_condition, decompose_univariate, evaluate_formula_at_exact_lifted_cell,
    lift_over_rational_sample, lift_two_variables, synthesize_cell_conditions, CellKind,
};
use quantifier_elimination::Polynomial;
use quantifier_elimination::{Formula, Relation};
use std::collections::BTreeMap;

#[test]
fn builds_sections_and_sectors_in_order() {
    let cells = decompose_univariate(&[UnivariatePolynomial::from_integers(&[-2, 0, 1])]);

    assert_eq!(cells.len(), 5);
    assert!(matches!(cells[0].kind, CellKind::Sector));
    assert!(matches!(cells[1].kind, CellKind::Section));
    assert!(matches!(cells[2].kind, CellKind::Sector));
    assert!(matches!(cells[3].kind, CellKind::Section));
    assert!(matches!(cells[4].kind, CellKind::Sector));
    assert!(cells.windows(2).all(|pair| pair[0].sample < pair[1].sample));
}

#[test]
fn returns_a_single_sector_without_roots() {
    let cells = decompose_univariate(&[UnivariatePolynomial::from_integers(&[1, 0, 1])]);
    assert_eq!(cells.len(), 1);
    assert!(matches!(cells[0].kind, CellKind::Sector));
}

#[test]
fn evaluates_formulas_at_algebraic_lifted_samples() {
    let base_cells = decompose_univariate(&[UnivariatePolynomial::from_integers(&[-2, 0, 1])]);
    let lifted_cells = decompose_univariate(&[UnivariatePolynomial::from_integers(&[1, 0, 1])]);
    // At (sqrt(2), 0), the polynomial x0^2 + x1 equals 2.
    assert!(evaluate_formula_at_exact_lifted_cell(
        &Formula::atom(
            Polynomial::variable(0).pow(2) + Polynomial::variable(1) - Polynomial::integer(2),
            Relation::Equal,
        ),
        0,
        1,
        &base_cells[3],
        &lifted_cells[0],
    )
    .unwrap());
}

#[test]
fn separates_distinct_roots_from_different_polynomials() {
    let cells = decompose_univariate(&[
        UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        UnivariatePolynomial::from_integers(&[-1, 1]),
    ]);
    let sections = cells
        .iter()
        .filter(|cell| matches!(cell.kind, CellKind::Section))
        .collect::<Vec<_>>();

    assert_eq!(sections.len(), 3);
    assert!(cells.windows(2).all(|pair| pair[0].sample < pair[1].sample));
}

#[test]
fn deduplicates_common_roots_from_different_polynomials() {
    let cells = decompose_univariate(&[
        UnivariatePolynomial::from_integers(&[-2, 0, 1]),
        UnivariatePolynomial::from_integers(&[-4, 0, 2]),
    ]);
    let sections = cells
        .iter()
        .filter(|cell| matches!(cell.kind, CellKind::Section))
        .count();

    assert_eq!(sections, 2);
}

#[test]
fn specializes_lower_variables_before_lifting() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let polynomial = x.clone() * x + y;
    let mut values = BTreeMap::new();
    values.insert(1, BigRational::from_integer((-2).into()));

    let cells = lift_over_rational_sample(&[polynomial], 0, &values);
    assert_eq!(cells.len(), 5);

    // Cell 0: (-infinity, -sqrt(2)), the left unbounded sector.
    assert!(matches!(cells[0].kind, CellKind::Sector));
    assert!(cells[0].root.is_none());
    assert!(cells[0].sample < BigRational::zero());

    // Cell 1: {-sqrt(2)}, a singleton section. The current representation
    // stores an isolating interval until exact algebraic samples are added.
    assert!(matches!(cells[1].kind, CellKind::Section));
    let first_root = cells[1].root.as_ref().expect("first section has a root");
    let first_exact = cells[1]
        .exact_sample
        .as_ref()
        .expect("first section has an exact sample");
    assert_eq!(first_exact.polynomial.to_string(), "x^2 - 2");
    assert_eq!(first_exact.interval, first_root.clone());
    assert!(first_root.lower < first_root.upper);
    assert!(first_root.lower <= cells[1].sample);
    assert!(cells[1].sample <= first_root.upper);
    assert!(cells[1].sample < BigRational::zero());

    // Cell 2: (-sqrt(2), sqrt(2)), the sector between the roots.
    assert!(matches!(cells[2].kind, CellKind::Sector));
    assert!(cells[2].root.is_none());
    assert!(cells[2].sample >= BigRational::zero());

    // Cell 3: {sqrt(2)}, a singleton section represented by an isolating
    // interval for now.
    assert!(matches!(cells[3].kind, CellKind::Section));
    let second_root = cells[3].root.as_ref().expect("second section has a root");
    let second_exact = cells[3]
        .exact_sample
        .as_ref()
        .expect("second section has an exact sample");
    assert_eq!(second_exact.polynomial.to_string(), "x^2 - 2");
    assert_eq!(second_exact.interval, second_root.clone());
    assert!(second_root.lower < second_root.upper);
    assert!(second_root.lower <= cells[3].sample);
    assert!(cells[3].sample <= second_root.upper);
    assert!(cells[3].sample >= BigRational::zero());

    // Cell 4: (sqrt(2), infinity), the right unbounded sector.
    assert!(matches!(cells[4].kind, CellKind::Sector));
    assert!(cells[4].root.is_none());
    assert!(cells[4].sample > BigRational::zero());
    assert!(cells.windows(2).all(|pair| pair[0].sample < pair[1].sample));
}

#[test]
fn evaluates_signs_and_relations_on_each_cell() {
    let polynomial = UnivariatePolynomial::from_integers(&[-2, 0, 1]);
    let cells = decompose_univariate(std::slice::from_ref(&polynomial));
    let signs = cells
        .iter()
        .map(|cell| cell.sign_of(&polynomial))
        .collect::<Vec<_>>();

    assert_eq!(signs, vec![1, 0, -1, 0, 1]);
    assert!(cells[1].satisfies(&polynomial, Relation::Equal));
    assert!(cells[2].satisfies(&polynomial, Relation::Less));
    assert!(cells[0].satisfies(&polynomial, Relation::Greater));
    assert!(!cells[3].satisfies(&polynomial, Relation::NotEqual));
}

#[test]
fn evaluates_boolean_formulas_on_cells() {
    let x = Polynomial::variable(0);
    let square_minus_two = x.clone() * x.clone() + Polynomial::integer(-2);
    let formula = Formula::And(vec![
        Formula::atom(square_minus_two, Relation::LessOrEqual),
        Formula::atom(x, Relation::NotEqual),
    ]);
    let cells = decompose_univariate(&[UnivariatePolynomial::from_integers(&[-2, 0, 1])]);

    let values = cells
        .iter()
        .map(|cell| cell.evaluate_formula(&formula).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values, vec![false, true, false, true, false]);
}

#[test]
fn builds_a_two_variable_lifting_layer() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let formula = Formula::exists(0, Formula::atom(x.clone() * x + y, Relation::Equal));
    let lifting = lift_two_variables(&formula, &[1, 0]).unwrap();

    assert_eq!(lifting.base_cells.len(), 3);
    assert_eq!(lifting.base_signs.len(), lifting.base_cells.len());
    assert!(lifting.base_signs.iter().all(|signs| !signs.is_empty()));
    assert_eq!(lifting.projection_stack.variable_order, vec![1, 0]);
    assert!(!lifting.base_polynomials.is_empty());
    assert_eq!(lifting.lifted_cells.len(), lifting.base_cells.len());
    assert!(lifting.lifted_cells.iter().all(|cells| !cells.is_empty()));
    assert_eq!(lifting.lifted_signs.len(), lifting.lifted_cells.len());
    assert!(lifting
        .lifted_signs
        .iter()
        .zip(&lifting.lifted_cells)
        .all(|(signs, cells)| signs.len() == cells.len()));

    let body = Formula::atom(
        Polynomial::variable(0) * Polynomial::variable(0) + Polynomial::variable(1),
        Relation::Equal,
    );
    let truth_table = lifting.truth_table(&body).unwrap();
    assert_eq!(truth_table.len(), lifting.base_cells.len());
    assert!(truth_table[0].iter().any(|value| *value));
    assert!(truth_table[1].iter().any(|value| *value));
    assert!(truth_table[2].iter().all(|value| !value));

    let conditions = lifting
        .base_cells
        .iter()
        .map(|cell| cell_condition(cell, &lifting.base_polynomials, 1))
        .collect::<Vec<_>>();
    assert_eq!(conditions.len(), lifting.base_cells.len());
}

#[test]
fn synthesizes_a_quantifier_free_formula_from_true_cells() {
    let polynomial = UnivariatePolynomial::from_integers(&[-2, 0, 1]);
    let cells = decompose_univariate(std::slice::from_ref(&polynomial));
    let truth_values = cells
        .iter()
        .map(|cell| cell.sign_of(&polynomial) < 0)
        .collect::<Vec<_>>();
    let formula =
        synthesize_cell_conditions(&cells, std::slice::from_ref(&polynomial), &truth_values, 0);

    let evaluated = cells
        .iter()
        .map(|cell| cell.evaluate_formula(&formula).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(evaluated, truth_values);
}
