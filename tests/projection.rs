use quantifier_elimination::cad::projection::{build_projection_stack, project, resultant};
use quantifier_elimination::{Formula, Polynomial, Relation};

#[test]
fn extracts_coefficients_and_derivatives() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let polynomial = x.clone() * x + y.clone();
    assert_eq!(polynomial.coefficient_in(0, 0), y);
    assert_eq!(polynomial.derivative(0).to_string(), "2*x0");
}

#[test]
fn computes_resultant_of_two_linear_polynomials() {
    let x = Polynomial::variable(0);
    let a = Polynomial::variable(1);
    let b = Polynomial::variable(2);
    let left = x.clone() + a;
    let right = x + b;
    assert_eq!(resultant(&left, &right, 0).to_string(), "x1 - x2");
}

#[test]
fn projection_contains_discriminant_and_resultant_data() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let first = x.clone() * x.clone() + y;
    let second = x + Polynomial::integer(1);
    let projected = project(&[first, second], 0);
    assert!(projected
        .iter()
        .any(|polynomial| polynomial.to_string() == "x1"));
    assert!(projected
        .iter()
        .any(|polynomial| polynomial.to_string() == "1"));
}

#[test]
fn builds_projection_levels_in_variable_order() {
    let x = Polynomial::variable(0);
    let y = Polynomial::variable(1);
    let formula = Formula::exists(0, Formula::atom(x.clone() * x + y, Relation::Equal));
    let stack = build_projection_stack(&formula, &[1, 0]);

    assert_eq!(stack.variable_order, vec![1, 0]);
    assert_eq!(stack.levels.len(), 3);
    assert!(!stack.levels[0].is_empty());
    assert!(!stack.levels[1].is_empty());
    assert!(!stack.levels[2].is_empty());
    assert!(stack.levels[2]
        .iter()
        .all(|polynomial| polynomial.variables().next().is_none()));
}
