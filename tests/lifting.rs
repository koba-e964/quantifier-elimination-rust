use quantifier_elimination::algebra::univariate::UnivariatePolynomial;
use quantifier_elimination::cad::lifting::{decompose_univariate, CellKind};

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
