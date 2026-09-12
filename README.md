# Quantifier Elimination

Exact symbolic quantifier elimination over the real numbers.

The crate currently provides the foundations of a cylindrical algebraic
decomposition (CAD) solver:

- rational-coefficient multivariate polynomials;
- exact univariate root isolation with Sturm sequences;
- algebraic real samples represented by defining polynomials and isolating intervals;
- Collins-style projection sets;
- one- and two-variable CAD lifting;
- Boolean formula evaluation and sign-condition synthesis.

## Example

```rust
use quantifier_elimination::{eliminate, Formula, Polynomial, Relation};

let x = Polynomial::variable(0);
let formula = Formula::exists(
    0,
    Formula::atom(x.clone() * x + Polynomial::integer(1), Relation::Equal),
);

assert_eq!(eliminate(&formula).unwrap(), Formula::False);
```

## Current scope

`eliminate` currently supports closed one-variable formulas and formulas with
one quantified variable plus one free variable. General multivariate formula
synthesis, exact substitution at algebraic lower-dimensional samples, and
full nested-quantifier elimination are still in progress.

All decision procedures use exact rational or algebraic representations;
floating-point arithmetic is not used for correctness decisions.

## Development

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
pre-commit run --all-files
```
