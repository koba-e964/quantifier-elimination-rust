use crate::algebra::univariate::{RootInterval, UnivariatePolynomial};
use crate::polynomial::{Polynomial, Variable};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellKind {
    Sector,
    Section,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnivariateCell {
    pub kind: CellKind,
    pub sample: BigRational,
    pub root: Option<RootInterval>,
}

impl UnivariateCell {
    fn sector(sample: BigRational) -> Self {
        Self {
            kind: CellKind::Sector,
            sample,
            root: None,
        }
    }

    fn section(root: RootInterval) -> Self {
        Self {
            sample: (&root.lower + &root.upper) / BigInt::from(2),
            kind: CellKind::Section,
            root: Some(root),
        }
    }
}

/// Build the one-dimensional CAD induced by a family of univariate
/// polynomials. Sections contain one real root; sectors contain no roots.
pub fn decompose_univariate(polynomials: &[UnivariatePolynomial]) -> Vec<UnivariateCell> {
    let mut roots = polynomials
        .iter()
        .flat_map(UnivariatePolynomial::isolate_real_roots)
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left.lower.cmp(&right.lower));
    roots.dedup_by(|left, right| left == right);

    if roots.is_empty() {
        return vec![UnivariateCell::sector(BigRational::zero())];
    }

    let mut cells = Vec::with_capacity(roots.len() * 2 + 1);
    cells.push(UnivariateCell::sector(
        &roots[0].lower - BigRational::from_integer(BigInt::from(1)),
    ));
    for (index, root) in roots.iter().cloned().enumerate() {
        cells.push(UnivariateCell::section(root.clone()));
        if let Some(next) = roots.get(index + 1) {
            cells.push(UnivariateCell::sector(
                (&root.upper + &next.lower) / BigInt::from(2),
            ));
        } else {
            cells.push(UnivariateCell::sector(
                &root.upper + BigRational::from_integer(BigInt::from(1)),
            ));
        }
    }
    cells
}

/// Specialize all variables other than `variable` at a rational sample point
/// and lift the resulting univariate family into cells.
pub fn lift_over_rational_sample(
    polynomials: &[Polynomial],
    variable: Variable,
    values: &std::collections::BTreeMap<Variable, BigRational>,
) -> Vec<UnivariateCell> {
    let specialized = polynomials
        .iter()
        .map(|polynomial| specialize_to_univariate(polynomial, variable, values))
        .collect::<Vec<_>>();
    decompose_univariate(&specialized)
}

fn specialize_to_univariate(
    polynomial: &Polynomial,
    variable: Variable,
    values: &std::collections::BTreeMap<Variable, BigRational>,
) -> UnivariatePolynomial {
    let mut coefficients = vec![BigRational::zero(); polynomial.degree(variable) + 1];
    for (monomial, coefficient) in polynomial.terms() {
        let mut value = coefficient.clone();
        for other_variable in monomial.variables() {
            if other_variable == variable {
                continue;
            }
            value *= values
                .get(&other_variable)
                .cloned()
                .unwrap_or_default()
                .pow(monomial.exponent(other_variable) as i32);
        }
        coefficients[monomial.exponent(variable)] += value;
    }
    UnivariatePolynomial::new(coefficients)
}
