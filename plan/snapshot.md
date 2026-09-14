# Snapshot Tests

Current step: Step 5 — finalize.

## Step 1: Move the implementation plan

- [x] Delete `IMPLEMENTATION_PLAN.md`.
- [x] Create this plan at `plan/snapshot.md`.
- [x] Validate and commit the plan migration as `bd1203d`.

## Step 2: Add snapshot-test infrastructure

- [x] Add `insta` as a development dependency.
- [x] Create stable snapshot helpers around canonical CLI/QE output.
- [x] Keep snapshots independent of hash-map ordering, `Debug` output, and midpoint-derived values.

## Step 3: Add representative snapshots

- [x] Snapshot closed existential and universal formulas.
- [x] Snapshot irrational-root and multivariate QE output.
- [x] Snapshot nested quantifier output.
- [x] Snapshot parser and CLI error output.
- [x] Include the multivariate example `exists x1. x1^2 + x0 + x2 = 0`.

## Step 4: Review and validation workflow

- [x] Verify `cargo test` fails on intentional snapshot changes using `INSTA_UPDATE=no`.
- [x] Document `cargo insta test` and `cargo insta review`.
- [x] Run formatting, tests, Clippy, and pre-commit.
- [x] Show the literal diff and commit each logical implementation slice as `cb7d698`.

## Step 5: Finalize

- [x] Review snapshot readability and remove unstable assertions.
- [ ] Push only after all snapshot work and validation are complete.
