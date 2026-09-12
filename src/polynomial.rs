use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

pub type Variable = usize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolynomialEvaluationError {
    MissingVariable(Variable),
    Arithmetic(crate::algebra::coefficient::ExactRealError),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VariableNames {
    names: BTreeMap<Variable, String>,
}

impl VariableNames {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, variable: Variable, name: impl Into<String>) {
        self.names.insert(variable, name.into());
    }

    pub fn with(mut self, variable: Variable, name: impl Into<String>) -> Self {
        self.insert(variable, name);
        self
    }

    pub fn name(&self, variable: Variable) -> String {
        self.names
            .get(&variable)
            .cloned()
            .unwrap_or_else(|| format!("x{}", variable))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Monomial(BTreeMap<Variable, usize>);

impl Monomial {
    pub fn one() -> Self {
        Self::default()
    }

    pub fn variable(variable: Variable) -> Self {
        let mut powers = BTreeMap::new();
        powers.insert(variable, 1);
        Self(powers)
    }

    pub fn exponent(&self, variable: Variable) -> usize {
        self.0.get(&variable).copied().unwrap_or(0)
    }

    pub fn variables(&self) -> impl Iterator<Item = Variable> + '_ {
        self.0.keys().copied()
    }

    pub fn total_degree(&self) -> usize {
        self.0.values().sum()
    }

    fn multiplied(&self, rhs: &Self) -> Self {
        let mut powers = self.0.clone();
        for (&variable, &power) in &rhs.0 {
            *powers.entry(variable).or_default() += power;
        }
        Self(powers)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Polynomial {
    terms: BTreeMap<Monomial, BigRational>,
}

impl Polynomial {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn one() -> Self {
        Self::constant(BigRational::one())
    }

    pub fn constant(value: BigRational) -> Self {
        if value.is_zero() {
            return Self::zero();
        }
        let mut terms = BTreeMap::new();
        terms.insert(Monomial::one(), value);
        Self { terms }
    }

    pub fn integer(value: i64) -> Self {
        Self::constant(BigRational::from_integer(BigInt::from(value)))
    }

    pub fn variable(variable: Variable) -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(Monomial::variable(variable), BigRational::one());
        Self { terms }
    }

    pub fn from_univariate(
        variable: Variable,
        polynomial: &crate::algebra::univariate::UnivariatePolynomial,
    ) -> Self {
        let mut result = Self::zero();
        for degree in 0..=polynomial.degree().unwrap_or(0) {
            let coefficient = polynomial.coefficient(degree);
            if coefficient.is_zero() {
                continue;
            }
            let mut powers = BTreeMap::new();
            if degree > 0 {
                powers.insert(variable, degree);
            }
            result.terms.insert(Monomial(powers), coefficient);
        }
        result
    }

    pub fn terms(&self) -> impl Iterator<Item = (&Monomial, &BigRational)> {
        self.terms.iter()
    }

    pub fn coefficient(&self, monomial: &Monomial) -> BigRational {
        self.terms.get(monomial).cloned().unwrap_or_default()
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn variables(&self) -> impl Iterator<Item = Variable> {
        self.terms
            .keys()
            .flat_map(Monomial::variables)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
    }

    pub fn degree(&self, variable: Variable) -> usize {
        self.terms
            .keys()
            .map(|monomial| monomial.exponent(variable))
            .max()
            .unwrap_or(0)
    }

    pub fn coefficient_in(&self, variable: Variable, degree: usize) -> Self {
        let mut result = Self::zero();
        for (monomial, coefficient) in &self.terms {
            if monomial.exponent(variable) != degree {
                continue;
            }
            let mut reduced = monomial.0.clone();
            reduced.remove(&variable);
            let entry = result.terms.entry(Monomial(reduced)).or_default();
            *entry += coefficient.clone();
        }
        result.terms.retain(|_, coefficient| !coefficient.is_zero());
        result
    }

    pub fn derivative(&self, variable: Variable) -> Self {
        let mut result = Self::zero();
        for (monomial, coefficient) in &self.terms {
            let exponent = monomial.exponent(variable);
            if exponent == 0 {
                continue;
            }
            let mut reduced = monomial.0.clone();
            if exponent == 1 {
                reduced.remove(&variable);
            } else {
                reduced.insert(variable, exponent - 1);
            }
            let term = coefficient * num_bigint::BigInt::from(exponent);
            let entry = result.terms.entry(Monomial(reduced)).or_default();
            *entry += term;
        }
        result.terms.retain(|_, coefficient| !coefficient.is_zero());
        result
    }

    pub fn pow(&self, exponent: usize) -> Self {
        (0..exponent).fold(Self::one(), |result, _| result * self.clone())
    }

    pub fn rename_variable(&self, old: Variable, new: Variable) -> Self {
        let mut result = Self::zero();
        for (monomial, coefficient) in &self.terms {
            let mut powers = monomial.0.clone();
            if let Some(exponent) = powers.remove(&old) {
                *powers.entry(new).or_default() += exponent;
            }
            let entry = result.terms.entry(Monomial(powers)).or_default();
            *entry += coefficient.clone();
        }
        result.terms.retain(|_, coefficient| !coefficient.is_zero());
        result
    }

    pub fn to_string_with(&self, names: &VariableNames) -> String {
        self.format_with(names)
    }

    fn format_with(&self, names: &VariableNames) -> String {
        if self.is_zero() {
            return "0".to_owned();
        }
        let mut output = String::new();
        let mut first = true;
        for (monomial, coefficient) in &self.terms {
            if !first && coefficient.is_positive() {
                output.push_str(" + ");
            } else if coefficient.is_negative() {
                output.push_str(if first { "-" } else { " - " });
            }
            let magnitude = coefficient.abs();
            if monomial.total_degree() == 0 || magnitude != BigRational::one() {
                output.push_str(&magnitude.to_string());
                if monomial.total_degree() != 0 {
                    output.push('*');
                }
            }
            let mut first_factor = true;
            for variable in monomial.variables() {
                if !first_factor {
                    output.push('*');
                }
                output.push_str(&names.name(variable));
                if monomial.exponent(variable) > 1 {
                    output.push('^');
                    output.push_str(&monomial.exponent(variable).to_string());
                }
                first_factor = false;
            }
            first = false;
        }
        output
    }

    pub fn evaluate(&self, values: &BTreeMap<Variable, BigRational>) -> BigRational {
        self.terms
            .iter()
            .fold(BigRational::zero(), |sum, (monomial, coefficient)| {
                let term = monomial
                    .variables()
                    .fold(coefficient.clone(), |value, variable| {
                        let exponent = monomial.exponent(variable);
                        let variable_value = values.get(&variable).cloned().unwrap_or_default();
                        value * variable_value.pow(exponent as i32)
                    });
                sum + term
            })
    }

    pub fn try_evaluate(&self, values: &BTreeMap<Variable, BigRational>) -> Option<BigRational> {
        if self
            .variables()
            .any(|variable| !values.contains_key(&variable))
        {
            None
        } else {
            Some(self.evaluate(values))
        }
    }

    pub fn evaluate_exact(
        &self,
        values: &BTreeMap<Variable, crate::algebra::coefficient::ExactReal>,
    ) -> Result<crate::algebra::coefficient::ExactReal, PolynomialEvaluationError> {
        self.terms.iter().try_fold(
            crate::algebra::coefficient::ExactReal::rational(BigRational::zero()),
            |sum, (monomial, coefficient)| {
                let term = monomial.variables().try_fold(
                    crate::algebra::coefficient::ExactReal::rational(coefficient.clone()),
                    |term, variable| {
                        let value = values
                            .get(&variable)
                            .ok_or(PolynomialEvaluationError::MissingVariable(variable))?;
                        (0..monomial.exponent(variable)).try_fold(term, |term, _| {
                            term.try_mul(value)
                                .map_err(PolynomialEvaluationError::Arithmetic)
                        })
                    },
                )?;
                sum.try_add(&term)
                    .map_err(PolynomialEvaluationError::Arithmetic)
            },
        )
    }
}

impl Add for Polynomial {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        for (monomial, coefficient) in rhs.terms {
            let entry = self.terms.entry(monomial.clone()).or_default();
            *entry += coefficient;
            if entry.is_zero() {
                self.terms.remove(&monomial);
            }
        }
        self
    }
}

impl Sub for Polynomial {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Mul for Polynomial {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = Self::zero();
        for (left_monomial, left_coefficient) in self.terms {
            for (right_monomial, right_coefficient) in &rhs.terms {
                let monomial = left_monomial.multiplied(right_monomial);
                let coefficient = left_coefficient.clone() * right_coefficient;
                let entry = result.terms.entry(monomial.clone()).or_default();
                *entry += coefficient;
                if entry.is_zero() {
                    result.terms.remove(&monomial);
                }
            }
        }
        result
    }
}

impl Neg for Polynomial {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            terms: self
                .terms
                .into_iter()
                .map(|(monomial, coefficient)| (monomial, -coefficient))
                .collect(),
        }
    }
}

impl fmt::Display for Polynomial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.format_with(&VariableNames::default()))
    }
}
