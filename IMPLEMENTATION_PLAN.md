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

## Step 7: Quantifier elimination extensions

- [ ] Support recursive lifting for multiple quantified variables.
- [ ] Support nested quantifiers with correct variable ordering and alpha-renaming.
- [ ] Generalize formula synthesis beyond one free variable.
- [ ] Preserve Boolean structure and simplify synthesized formulas.
- [ ] Add semantic validation over rational and algebraic assignments.

## Validation required for each completed logical step

- [x] `cargo fmt --all`
- [x] `cargo test`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `pre-commit run --all-files`
