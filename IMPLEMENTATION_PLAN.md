# Exact CAD Lifting Implementation Plan

This checklist breaks the repository task list into reviewable substeps. Each
logical step is completed as a unit; approval is requested before its commit.

## Step 1: Algebraic root-sample model

- [x] Add `AlgebraicRootSample` refinement with defining-polynomial preservation.
- [x] Add exact ordering for disjoint samples and same-polynomial roots.
- [x] Return explicit errors for unresolved cross-polynomial comparisons.
- [x] Add refinement, ordering, and unsupported-common-root tests.
- [x] Commit as `636318d`.

## Step 2: Algebraic-coefficient root isolation

- [x] Route rational-coefficient cases through Sturm isolation.
- [x] Handle repeated rational roots as distinct roots.
- [x] Validate root ordering and deduplication behavior.
- [x] Commit as `7b82e31`.

## Step 3: Integrate algebraic root samples into CAD lifting

- [x] Add algebraic root-sample cells and algebraic univariate decomposition.
- [x] Use exact root-sample comparison for ordering and deduplication.
- [x] Support quadratic lifting over irrational algebraic base sections.
- [x] Commit the combined lifting work as `89abc8c`.

## Step 4: Exact formula evaluation

- [x] Prevent midpoint fallback for unsupported algebraic root samples.
- [x] Add exact signs for samples with rational defining polynomials.
- [x] Evaluate atomic relations at supported algebraic sections.
- [x] Cover equality, disequality, strict/non-strict relations, `Not`, `And`, and `Or`.
- [x] Commit implementation as `674d7a4` and semantic coverage as `fabeafd`.

## Step 5: Scope, errors, and end-to-end coverage

- [x] Replace the old quadratic-root limitation with generic supported-degree dispatch.
- [x] Add an end-to-end quantified quadratic example over an irrational base section.
- [x] Remove the obsolete `AlgebraicBaseSampleUnsupported` error.
- [x] Update README scope and remaining limitation documentation.
- [x] Obtain approval and commit the complete Step 5 change as `d70b725`.

## Step 6: Normalization and canonicalization

- [x] Add reusable square-free and monic normalization for rational polynomials.
- [x] Normalize operation-derived defining polynomials where practical.
- [x] Collapse proven rational `ExactReal` results, including endpoint roots.
- [x] Preserve and test exact-cancellation fast paths.
- [x] Replace structural equality with semantic equality where required.
- [x] Obtain approval and commit the complete Step 6 change as `33fce3c`.

## Step 7a: Recursive nested-quantifier dispatch

- [x] Recursively eliminate inner quantifiers before the enclosing quantifier.
- [x] Preserve Boolean structure while reducing nested quantified children.
- [x] Add a semantic nested-quantifier regression test.
- [x] Obtain approval and commit this Step 7a change as `da21133`.

## Step 7b: Scope and Boolean-branch regression coverage

- [x] Verify nested shadowed-variable scopes.
- [x] Verify quantified branches inside Boolean combinations.
- [x] Preserve the existing implementation after the invariant audit.
- [x] Obtain approval and commit this Step 7b change as `e1a8f26`.

## Step 7c: Multivariate synthesis boundary

- [x] Confirm the current two-variable lifting/synthesis boundary.
- [x] Document rejection of formulas with multiple free variables.
- [x] Retain the explicit multivariate-rejection regression test.
- [x] Obtain approval and commit this Step 7c change as `f6305b3`.

## Step 7d: Boolean synthesis normalization

- [x] Collapse singleton cell conditions to atoms.
- [x] Collapse singleton synthesized disjunctions to their condition.
- [x] Add a formula-shape regression test.
- [x] Obtain approval and commit this Step 7d change as `b05abd2`.

## Step 7e: Algebraic-assignment semantic validation

- [x] Evaluate synthesized quantifier-free results on exact algebraic base cells.
- [x] Cover both real roots and surrounding sectors.
- [x] Add an end-to-end semantic regression test.
- [x] Obtain approval and commit this Step 7e change as `08db6e2`.

## Step 7: Quantifier elimination extensions

- [ ] Support recursive lifting for multiple quantified variables.
- [ ] Support nested quantifiers with correct variable ordering and alpha-renaming.
- [ ] Generalize formula synthesis beyond one free variable.
- [ ] Preserve Boolean structure and simplify synthesized formulas.
- [ ] Add semantic validation over rational and algebraic assignments.

## Step 8: Parser and CLI

- [x] Define the initial textual grammar for variables, polynomials, relations, Boolean connectives, and quantifiers.
- [x] Implement a parser with location-aware, user-facing errors.
- [x] Add a CLI binary that parses a formula and prints the eliminated result.
- [x] Support input from an argument and from standard input.
- [x] Add parser unit tests and CLI integration tests.
- [x] Document examples and unsupported syntax.
- [x] Parser foundation committed as `cad9bfa`; CLI and documentation follow in Steps 8b and 8c.
- [x] Complete the parser/CLI task across commits `cad9bfa`, `e4b138f`, `b277013`, and `b755aad`.

## Step 8b: CLI wrapper

- [x] Add a `qe` binary.
- [x] Accept a formula argument or standard input.
- [x] Print a stable textual result.
- [x] Add CLI success and error integration tests.
- [x] Obtain approval and commit the CLI substep as `7f5f90c`.

## Step 8c: Parser and CLI documentation

- [x] Document the accepted grammar.
- [x] Document CLI argument and standard-input usage.
- [x] Document unsupported syntax and current elimination limits.
- [x] Obtain approval and commit the documentation substep as `b277013`.

## Step 9a: Cross-polynomial algebraic root comparison

- [x] Detect common roots from different rational defining polynomials with an exact gcd.
- [x] Handle overlapping intervals and endpoint roots without midpoint fallback.
- [x] Add regression coverage for scaled defining polynomials.
- [x] Commit this root-sample hardening substep as `05fae6f`.

## Step 9b: Endpoint-aware algebraic root refinement

- [x] Detect exact roots on isolating interval boundaries.
- [x] Refine boundary roots toward the endpoint without dropping them.
- [x] Add endpoint refinement regression coverage.
- [x] Commit this root-sample hardening substep as `8fce56a`.

## Step 9c: Repeated algebraic-coefficient roots

- [x] Detect repeated quadratic roots through exact derivative evaluation.
- [x] Isolate the repeated `√2` root without relying on sign variation.
- [x] Add regression coverage for an even-multiplicity algebraic root.
- [x] Commit this root-sample hardening substep as `3bac0a0`.

## Step 9d: Arbitrary-degree algebraic root coverage

- [x] Cover quadratic root isolation with an algebraic constant term.
- [x] Validate both real roots of `y² - √2`.
- [x] Commit this root-isolation coverage substep as `dd10136`.

## Step 9e: No-real-root algebraic coverage

- [x] Cover a positive algebraic constant with no real quadratic roots.
- [x] Preserve the exact empty-root result.
- [x] Commit this root-isolation coverage substep as `2fb96b6`.

## Step 10a: Vacuous multivariate quantifiers

- [x] Eliminate quantifiers whose bound variable is absent from the body.
- [x] Preserve bodies with multiple free variables without invoking CAD synthesis.
- [x] Add existential and universal regression coverage.
- [x] Commit this higher-dimensional QE substep as `5086aa3`.

## Step 10b: Boolean simplification before multivariate dispatch

- [x] Normalize Boolean structure after recursively reducing nested quantifiers.
- [x] Collapse dead quantified branches before free-variable counting.
- [x] Add existential and universal multivariate regression coverage.
- [x] Commit this higher-dimensional QE substep as `d5f58c9`.

## Step 10c: Nested closed-quantifier regression coverage

- [x] Cover recursive elimination across two quantified variables.
- [x] Cover existential/universal nesting that reduces to true.
- [x] Cover universal/existential nesting that reduces to false.
- [x] Commit this higher-dimensional QE substep as `73368d5`.

## Step 10d: Linear atomic multivariate elimination

- [x] Extract quantified-variable coefficients for linear atomic formulas.
- [x] Eliminate all six relation kinds for existential and universal quantifiers.
- [x] Add semantic coverage with two remaining free variables.
- [x] Commit this higher-dimensional QE substep as `2b8ca09`.

## Step 10e: Linear relation-matrix regression coverage

- [x] Cover all six relations for existential linear elimination.
- [x] Cover all six relations for universal linear elimination.
- [x] Verify zero and nonzero leading-coefficient cases semantically.
- [ ] Commit this higher-dimensional QE substep.

## Validation required for each completed logical step

- [x] `cargo fmt --all`
- [x] `cargo test`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `pre-commit run --all-files`
