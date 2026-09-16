# Automatic variable elimination order

Current step: Step 3 — implement adaptive exact selection.

## Objective

Automatically choose a free-variable lifting order that avoids unsupported
algebraic-root lifting when an equivalent order is available, while preserving
exact results and keeping explicit `--variable-order` behavior reproducible.

## Step 1: Reproduce and characterize order sensitivity [x]

- [x] Record the reported formula and both explicit orders as bounded CLI
  regressions to investigate:
  - `exists t. y = t*x+t^2` with `x,y`.
  - `exists t. y = t*x+t^2` with `y,x`.
- [x] Run the same formula without `--variable-order` and record the selected
  order and result: the default path selects `x -> y -> t` and returns
  `4*y + x^2 >= 0`.
- [x] Trace why the `y,x,t` lifting path reaches
  `AlgebraicRootSampleUnsupported` while `x,y,t` succeeds.
- [x] Identify order-quality signals that are available before lifting:
  quantified-variable degree and mixed quantified/free monomial coupling are
  available in the polynomial structure; algebraic section creation and the
  cell budget remain runtime signals for a later fallback slice.

Finding: named free variables are assigned by first appearance, so the
reported formula initially scores as `y = 0`, `x = 1`. The old tie-breaker
therefore selected `y -> x -> t`; the successful order is `x -> y -> t`
because `t*x` is a mixed quantified/free monomial while `y` is additive.

## Step 2: Define automatic-order selection [-]

- [x] Specify a deterministic candidate-order policy for every recursive
  quantifier-elimination call.
- [x] Prefer orders that keep the quantified variable’s defining polynomial
  univariate over rational samples before introducing algebraic coefficient
  sections.
- [x] Preserve the existing score-based selector as the deterministic
  tie-breaker.
- [x] Define that automatic selection may try a second exact order after
  `AlgebraicRootSampleUnsupported`; budget rejection remains a separate
  decision because it can indicate a genuinely over-large problem.
- [x] Keep explicit `--variable-order` as a strict user-requested order; do
  not silently reorder it.

## Step 3: Implement adaptive exact selection

- [x] Add the order-selection/adaptation logic in the quantifier evaluation
  layer rather than in the CLI parser.
- [x] Ensure every candidate attempt uses exact arithmetic and does not reuse
  partial CAD state from a failed order.
- [x] Record rejected automatic alternatives and the selected order in stats
  when `--stats` is enabled; ordinary one-attempt stats remain unchanged.
- [ ] Return the original error with useful order context when no candidate is
  supported.

## Step 4: Regression and semantic coverage

- [x] Add CLI coverage showing the default order succeeds for the reported
  formula.
- [x] Keep explicit `x,y` and `y,x` tests: the explicit-order contract and
  their documented outcomes must remain clear.
- [x] Cover the deterministic candidate policy and strict explicit-order
  behavior. No stable end-to-end formula currently exercises a successful
  automatic retry after the improved scorer selects its preferred order.
- [ ] Compare automatic and explicit successful results semantically at exact
  rational and algebraic assignments.
- [ ] Add a bounded regression for a formula where all candidate orders remain
  unsupported, ensuring failure is deterministic and informative.

## Step 5: Documentation

- [x] Document the automatic-order policy and fallback behavior in `README.md`.
- [x] Document that `--variable-order` disables automatic reordering and is
  intended for reproducible experiments.
- [x] Document the new stats fields, if any, for selected and attempted orders.
