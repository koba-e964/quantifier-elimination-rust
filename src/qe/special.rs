use crate::formula::{Atom, Formula, Quantifier, Relation};
use crate::polynomial::Polynomial;
use crate::qe::simplify::simplify;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpecialRule {
    SymmetricSumProduct,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecialHandlingConfig {
    pub enabled_rules: Vec<SpecialRule>,
}

impl Default for SpecialHandlingConfig {
    fn default() -> Self {
        Self {
            enabled_rules: vec![SpecialRule::SymmetricSumProduct],
        }
    }
}

impl SpecialHandlingConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read special-rules config {}: {error}",
                path.display()
            )
        })?;
        Self::parse(&contents)
    }

    pub fn parse(contents: &str) -> Result<Self, String> {
        let mut config = Self {
            enabled_rules: Vec::new(),
        };
        let mut in_special_rules = false;
        for (line_number, raw_line) in contents.lines().enumerate() {
            let line = raw_line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                in_special_rules = &line[1..line.len() - 1] == "special-rules";
                if !in_special_rules {
                    return Err(format!(
                        "unknown special-rules config section on line {}",
                        line_number + 1
                    ));
                }
                continue;
            }
            if !in_special_rules {
                return Err(format!(
                    "special-rules config entry appears outside [special-rules] on line {}",
                    line_number + 1
                ));
            }
            let Some((name, value)) = line.split_once('=') else {
                return Err(format!(
                    "expected rule = true|false on line {}",
                    line_number + 1
                ));
            };
            if name.trim() != "symmetric-sum-product" {
                return Err(format!(
                    "unknown special rule `{}` on line {}",
                    name.trim(),
                    line_number + 1
                ));
            }
            match value.trim() {
                "true" => config.enabled_rules.push(SpecialRule::SymmetricSumProduct),
                "false" => {}
                _ => {
                    return Err(format!(
                        "special rule `{}` must be true or false on line {}",
                        name.trim(),
                        line_number + 1
                    ));
                }
            }
        }
        Ok(config)
    }
}

pub(crate) fn rewrite_symmetric_pair(formula: &Formula) -> Option<Formula> {
    let Formula::Quantified {
        quantifier: Quantifier::Exists,
        variable: left,
        body,
    } = formula
    else {
        return None;
    };
    let Formula::Quantified {
        quantifier: Quantifier::Exists,
        variable: right,
        body,
    } = body.as_ref()
    else {
        return None;
    };
    if left == right {
        return None;
    }
    let atoms = match body.as_ref() {
        Formula::Atom(atom) => vec![atom],
        Formula::And(formulas) => formulas
            .iter()
            .map(|formula| match formula {
                Formula::Atom(atom) => Some(atom),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?,
        _ => return None,
    };
    let free_variables = body
        .free_variables()
        .into_iter()
        .filter(|variable| variable != left && variable != right)
        .collect::<Vec<_>>();
    let sum_aliases = free_variables
        .iter()
        .copied()
        .filter(|alias| {
            atoms
                .iter()
                .any(|atom| is_sum_binding(atom, *left, *right, *alias))
        })
        .collect::<Vec<_>>();
    let product_aliases = free_variables
        .iter()
        .copied()
        .filter(|alias| {
            atoms
                .iter()
                .any(|atom| is_product_binding(atom, *left, *right, *alias))
        })
        .collect::<Vec<_>>();
    let [sum_alias] = sum_aliases.as_slice() else {
        return None;
    };
    if product_aliases.len() > 1 {
        return None;
    }
    let product_alias = product_aliases.first().copied();
    let product_variable = product_alias.unwrap_or_else(|| next_variable(formula));

    let mut rewritten = Vec::new();
    for atom in atoms {
        if is_sum_binding(atom, *left, *right, *sum_alias)
            || product_alias.is_some_and(|alias| is_product_binding(atom, *left, *right, alias))
        {
            continue;
        }
        let polynomial =
            atom.polynomial
                .rewrite_symmetric(*left, *right, *sum_alias, product_variable)?;
        rewritten.push(Formula::atom(polynomial, atom.relation));
    }
    let discriminant = Polynomial::variable(*sum_alias).pow(2)
        - Polynomial::variable(product_variable) * Polynomial::integer(4);
    rewritten.push(Formula::atom(discriminant, Relation::GreaterOrEqual));
    let result = simplify(&Formula::And(rewritten));
    Some(if product_alias.is_some() {
        result
    } else {
        Formula::exists(product_variable, result)
    })
}

fn next_variable(formula: &Formula) -> usize {
    all_variables(formula)
        .into_iter()
        .max()
        .map_or(0, |variable| variable + 1)
}

fn all_variables(formula: &Formula) -> Vec<usize> {
    match formula {
        Formula::True | Formula::False => Vec::new(),
        Formula::Atom(atom) => atom.polynomial.variables().collect(),
        Formula::Not(body) => all_variables(body),
        Formula::And(formulas) | Formula::Or(formulas) => {
            formulas.iter().flat_map(all_variables).collect()
        }
        Formula::Quantified { variable, body, .. } => std::iter::once(*variable)
            .chain(all_variables(body))
            .collect(),
    }
}

fn is_sum_binding(atom: &Atom, left: usize, right: usize, alias: usize) -> bool {
    if atom.relation != Relation::Equal {
        return false;
    }
    let expected =
        Polynomial::variable(alias) - Polynomial::variable(left) - Polynomial::variable(right);
    atom.polynomial == expected || atom.polynomial == -expected
}

fn is_product_binding(atom: &Atom, left: usize, right: usize, alias: usize) -> bool {
    if atom.relation != Relation::Equal {
        return false;
    }
    let expected =
        Polynomial::variable(alias) - Polynomial::variable(left) * Polynomial::variable(right);
    atom.polynomial == expected || atom.polynomial == -expected
}

#[cfg(test)]
mod tests {
    use super::{rewrite_symmetric_pair, SpecialHandlingConfig, SpecialRule};
    use crate::formula::{Formula, Relation};
    use crate::polynomial::Polynomial;

    #[test]
    fn parses_enabled_symmetric_rule() {
        let config =
            SpecialHandlingConfig::parse("[special-rules]\nsymmetric-sum-product = true\n")
                .unwrap();

        assert_eq!(config.enabled_rules, vec![SpecialRule::SymmetricSumProduct]);
    }

    #[test]
    fn parses_disabled_symmetric_rule() {
        let config =
            SpecialHandlingConfig::parse("[special-rules]\nsymmetric-sum-product = false\n")
                .unwrap();

        assert!(config.enabled_rules.is_empty());
    }

    #[test]
    fn rejects_unknown_rules() {
        let error =
            SpecialHandlingConfig::parse("[special-rules]\nunknown-rule = true\n").unwrap_err();

        assert!(error.contains("unknown special rule"));
    }

    #[test]
    fn rejects_non_symmetric_polynomials() {
        let x0 = Polynomial::variable(0);
        let x1 = Polynomial::variable(1);
        let x2 = Polynomial::variable(2);
        let x3 = Polynomial::variable(3);
        let x4 = Polynomial::variable(4);
        let formula = Formula::exists(
            0,
            Formula::exists(
                1,
                Formula::And(vec![
                    Formula::atom(x2.clone() - x0.clone() - x1.clone(), Relation::Equal),
                    Formula::atom(x3 - x0.clone() * x1.clone(), Relation::Equal),
                    Formula::atom(x4 - x0.clone() * x0 - x1, Relation::Equal),
                ]),
            ),
        );

        assert!(rewrite_symmetric_pair(&formula).is_none());
    }
}
