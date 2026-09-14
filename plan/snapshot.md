# Snapshot Tests

Current step: Step 2 — snapshot-test setup.

## Step 1: Move the implementation plan

- [x] Delete `IMPLEMENTATION_PLAN.md`.
- [x] Create this plan at `plan/snapshot.md`.
- [ ] Validate and commit the plan migration.

## Step 2: Add snapshot-test infrastructure

- [ ] Add `insta` as a development dependency.
- [ ] Create stable snapshot helpers around canonical CLI/QE output.
- [ ] Keep snapshots independent of hash-map ordering, `Debug` output, and midpoint-derived values.

## Step 3: Add representative snapshots

- [ ] Snapshot closed existential and universal formulas.
- [ ] Snapshot irrational-root and multivariate QE output.
- [ ] Snapshot nested quantifier output.
- [ ] Snapshot parser and CLI error output.
- [ ] Include the multivariate example `exists x1. x1^2 + x0 + x2 = 0`.

## Step 4: Review and validation workflow

- [ ] Verify `cargo test` fails on intentional snapshot changes.
- [ ] Document `cargo insta test` and `cargo insta review`.
- [ ] Run formatting, tests, Clippy, and pre-commit.
- [ ] Show the literal diff and commit each logical implementation slice.

## Step 5: Finalize

- [ ] Review snapshot readability and remove unstable assertions.
- [ ] Push only after all snapshot work and validation are complete.
