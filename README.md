# Quantifier Elimination

Exact symbolic quantifier elimination over the real numbers.

The crate currently provides the foundations of a cylindrical algebraic
decomposition (CAD) solver:

- rational-coefficient multivariate polynomials;
- exact univariate root isolation with Sturm sequences;
- algebraic real samples represented by defining polynomials and isolating intervals;
- algebraic-coefficient root samples with exact refinement, ordering, and deduplication;
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

## Parser and CLI

The `qe` command accepts a formula as one argument or reads it from standard
input:

```text
qe 'exists x0. x0^2 + 1 = 0'
printf '%s' 'exists x0. x0 = 0' | qe
```

The initial grammar uses variables such as `x0`, integer constants, `+`, `-`,
`*`, `^`, parentheses, comparison operators (`=`, `!=`, `<`, `<=`, `>`,
`>=`), Boolean operators (`!`, `&&`, `||`), and quantifiers written as
`exists x0. FORMULA` or `forall x0. FORMULA`.

The CLI currently reports quantifier-free results as `true`, `false`, or a
formula using the same comparison and Boolean syntax. Floating-point literals,
implicit multiplication, and named variables are not supported yet.

## Current scope

`eliminate` supports closed one-variable formulas, formulas with one quantified
variable plus one free variable, vacuous quantifiers, and single atomic formulas
linear in the quantified variable even when multiple free variables remain.
General multivariate synthesis for nonlinear or compound formulas is not
supported yet.

Two-variable lifting supports quadratic sections over irrational algebraic base
samples. Formula relations at lifted algebraic sections are evaluated exactly
when the section has a rational defining polynomial. Operations that require
general arithmetic or root comparison over algebraic coefficients still return
explicit unsupported-operation errors.

All decision procedures use exact rational or algebraic representations;
floating-point arithmetic is not used for correctness decisions.

## Development

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
pre-commit run --all-files
```
