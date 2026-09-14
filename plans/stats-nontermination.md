# Stats and non-terminating simple formulas

Current step: Step 3 — load typed special-rule declarations from a configuration file.

## Observed issue

Intended reproduction:

```text
cargo run -r --bin qe -- 'exists x0. exists x1. x2=x0+x1&&x3=x0*x1&&x4=x0^2+x1^2' --stats
```

The corrected formula produces no output after a bounded wait in release mode.
The suspected path is `eliminate_recursive` handling the inner `exists x1`
with free variables `x2`, `x3`, and `x4`, then entering recursive CAD lifting
with variable order `[x2, x3, x4, x1]` before the outer quantifier can exploit
the sum/product/square relationship.

## Step 1: Reproduce and localize

- [x] Run the command exactly as pasted and confirm the escaped `\*` is a parse error.
- [x] Run the intended formula with `x0*x1` and `--stats` in release mode.
- [x] Confirm the intended formula produces no output after a bounded wait.
- [ ] Identify which nested-elimination branch and CAD lifting level dominate.

## Step 2: Control elimination order

- [x] Separate quantified-variable order from free-variable order in the
  recursive CAD dispatcher.
- [x] Record the current order and candidate orders in the stats report.
- [x] Define a deterministic order-selection policy that prioritizes the
  variables constrained by the current quantified body and avoids unnecessary
  lifting dimensions.
- [x] Add an explicit order override for reproducible experiments.
- [x] Verify that changing the order preserves exact semantic results.

## Step 3: Add special handling for simple cases

- [x] Implement and test a generic exact reducer from symmetric polynomials in
  `x0`, `x1` to sum/product variables.
- [x] Add `special-handling` as an option with default `true`, plus an explicit
  way to disable it for baseline CAD comparisons.
- [x] Detect the Vieta pattern
  `exists x0. exists x1. (...)` with
  `s = x0 + x1` and `p = x0*x1` (including equivalent normalized forms), then
  reduce every symmetric polynomial in `x0` and `x1` to a polynomial in `s`
  and `p`. For example, `x0^2 + x1^2` becomes `s^2 - 2*p`, while
  `x0^3 + x1^3` becomes `s^3 - 3*p*s`.
- [x] Always emit the real-root condition `s^2 - 4*p >= 0` when eliminating
  existential witnesses `x0` and `x1`.
- [x] Treat existential quantification of both witness variables as a required
  rule precondition; do not apply the elimination rewrite to free or universal
  `x0`/`x1` variables.
- [x] Reject or fall back cleanly for non-symmetric expressions; do not apply a
  partial rewrite to a polynomial that changes under swapping `x0` and `x1`.
- [x] Keep special rules guarded: unsupported shapes must fall back to the
  general exact elimination path.
- [ ] Design a rule representation that can be serialized as a configuration
  file if the rule language remains unambiguous and type-safe; otherwise keep
  the matcher implementation-driven and expose only configuration for enabling
  and ordering rules.
- [x] Move special-rule implementations behind a dedicated module or directory
  boundary; evaluate a separate crate only if the dependency and API boundary
  remains clean.
- [ ] Load the typed rule declarations from a user-provided configuration file
  and expose the file path through the CLI.
- [ ] Prefer configuration that declares symmetric witness groups and their
  sum/product bindings; keep the mathematically sensitive symmetric-polynomial
  reduction engine typed and implementation-backed rather than accepting
  arbitrary user-authored code or unchecked replacement strings.
- [ ] Document precedence, matching conditions, generated constraints, and
  whether a rule is allowed to call the CAD verifier.

## Step 4: Verify special rules against CAD

- [ ] Run each special rule with handling enabled and disabled on the same
  formula.
- [ ] Compare results semantically, not by formula string: use exact CAD cell
  evaluation or an equivalent exact oracle.
- [ ] Add positive, negative, renamed-variable, reordered-conjunction, and
  non-matching regression cases.
- [ ] Record whether the verifier is a test-only oracle or an optional runtime
  safety check.

## Step 5: Define the termination guard

- [ ] Add a bounded diagnostic mode or progress counters that distinguish recursive calls, projection levels, and cells constructed.
- [ ] Establish a reproducible threshold or timeout test for the formula.
- [ ] Preserve exact arithmetic; do not replace the CAD path with approximate sampling.

## Step 6: Regression coverage

- [ ] Add a bounded CLI regression test for the reported formula with special
  handling enabled and disabled.
- [ ] Add snapshots for both result modes and requested stats once the output
  is stable.
- [ ] Run formatting, tests, Clippy, and pre-commit.

## Step 7: Review and commit

- [x] Show the literal diff for each logical implementation slice.
- [x] Wait for approval before committing in commit+review mode.
- [x] Record implementation commit `991e3f4` for special handling.
- [x] Record implementation commit `4a99f43` for elimination-order reporting and override support.
- [x] Record implementation commit `6f906d9` for deterministic default order selection.
- [x] Record implementation commit `239eef7` for alternate-order semantic verification.
- [x] Record implementation commit `e3809e0` for the dedicated special-rule module.
