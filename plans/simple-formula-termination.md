# Termination for very simple formulae

Current step: Complete.

## Scope

Investigate why simple existential formulas can run indefinitely, including:

- the quadratic sum/product formula with special handling explicitly disabled
  and an explicit free-variable order;
- the cubic symmetric relation `x^3 + y^3 = 3*x*y` together with the free
  sum `k = x + y`.

## Step 1: Reproduce and localize

- [x] Capture bounded reproductions for both commands without allowing an
  unbounded test run.
- [x] Identify the recursive quantifier-elimination branch and lifting order
  reached by each formula.
- [x] Compare the explicit order `x2,x3,x4` with the automatically selected
  order and record the source of the blow-up.

Findings: both reported commands exceed a five-second bounded run. The
quadratic formula terminates immediately when the existing symmetric rule is
enabled, but disabling it sends the inner quantifier through recursive CAD.
The explicit top-level order is discarded there because the inner body also
contains the outer witness. The cubic formula does not match the current rule
because it has a sum binding but no explicit product binding.

## Step 2: Generalize symmetric reduction

- [x] Determine whether the cubic relation is recognized as symmetric even
  though it has a sum binding but no explicit product binding.
- [x] Match `exists x. exists y.` formulas with one sum binding, no product
  binding, and remaining atoms symmetric in `x` and `y`.
- [x] Introduce a fresh `p` and rewrite every matching atom using
  `s = x + y`, `p = x*y`, and the real-root condition `s^2 - 4*p >= 0`.
- [x] Substitute the sum binding into the rewritten formula and eliminate only
  `exists p.` with an exact linear-in-`p` sign split.
- [x] Preserve the general CAD fallback when the rewrite does not match or
  cannot be applied safely.

Design target: replace the existential witnesses `x` and `y` with their sum
`s` and a fresh existential product variable `p`:

```text
exists p. (s^3 - 3*p*(s + 1) = 0) && (s^2 - 4*p >= 0)
```

The first atom is the rewritten symmetric equation, and the second atom is the
real-root condition for `t^2 - s*t + p`. For the reported formula, the binding
`k = x + y` substitutes `s = k`, so the intermediate formula is:

```text
exists p. (k^3 - 3*p*(k + 1) = 0) && (k^2 - 4*p >= 0)
```

Eliminating only this one-dimensional product variable gives `-1 < k <= 3`,
including the exceptional-factor analysis at `k = -1`.

Implementation touchpoints: the symmetric matcher and latent-product rewrite
belong in `src/qe/special.rs`; dispatch must invoke the existing exact
one-variable elimination path from `src/qe/evaluate.rs`; recursive order
precedence belongs in `select_variable_order`; and the complexity budget must
be checked inside `src/cad/lifting.rs`, before a complete recursive lift is
returned.

## Step 3: Make the baseline terminate predictably

- [x] Treat `--variable-order=a,b,...` as precedence: use listed variables
  present in each recursive free-variable set, then append omitted variables
  using the deterministic default selector.
- [x] Add a bounded CAD complexity guard that reports the quantified variable,
  lifting order, projection level, and cell count when a case exceeds its
  configured budget.
- [x] Ensure special handling disabled remains a valid exact baseline and does
  not silently use a special rewrite.

## Step 4: Regression coverage

- [x] Add a bounded CLI regression for `exists x0. exists x1.
  x2=x0+x1 && x3=x0*x1 && x4=x0^2+x1^2` with
  `--special-handling=false --variable-order=x2,x3,x4`.
- [x] Add a bounded CLI regression for `exists x. exists y.
  x^3+y^3=3*x*y && k=x+y`.
- [x] Add a snapshot for `exists x. exists y. x+y=k &&
  x^2+x*y+y^2=1`, which implicitly determines `p = k^2 - 1` and should
  reduce to the real-root condition `4 - 3*k^2 >= 0`.
- [x] Check semantic equivalence between `exists x. exists y.
  x^3+y^3=3*x*y && k=x+y` and `exists p. (k^3-3*p*(k+1)=0) &&
  (k^2-4*p>=0)` at `k = -1`, `k = 0`, and representative values.
- [x] Add stable snapshots for the resulting simplified formulas.
