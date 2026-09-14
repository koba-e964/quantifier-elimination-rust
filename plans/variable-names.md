# Ergonomic variable names

Current step: Step 2 — add lexical name resolution.

## Step 1: Add a named-parser API

- [x] Add a `ParsedFormula` result containing the numeric formula and display-name mapping.
- [x] Add `parse_formula_with_names` while preserving legacy `parse_formula` behavior.
- [x] Keep the solver internals numeric; resolve names only at the parser/CLI boundary.

## Step 2: Add lexical name resolution

- [ ] Accept identifiers such as `x`, `y`, `st`, and `tmp` as variables.
- [ ] Resolve quantified variables with lexical scopes and fresh internal IDs.
- [ ] Preserve correct behavior for shadowed names.
- [ ] Reject reserved words (`exists`, `forall`, `true`, and `false`) as variable names.

## Step 3: Preserve names in output

- [ ] Format quantifier-free formulas with parsed variable names.
- [ ] Format lifting orders with parsed names.
- [ ] Keep numeric fallback names (`xN`) for unnamed/internal variables.

## Step 4: Make variable-order overrides name-aware

- [ ] Resolve `--variable-order=st,p,x,y` through the parsed free-variable names.
- [ ] Preserve compatibility with names such as `x2`.
- [ ] Reject unknown, bound, duplicate, or incomplete names using user-facing names.

## Step 5: Tests, snapshots, and documentation

- [ ] Add parser tests for named, quantified, nested, and shadowed variables.
- [ ] Add CLI tests for named output and named elimination orders.
- [ ] Add snapshots while preserving existing `x0`, `x1`, ... snapshots.
- [ ] Document named variables in `README.md` and `syntax.md`.
- [ ] Run formatting, tests, Clippy, and pre-commit for each implementation slice.

## Implementation commits

- [x] Plan created and committed as `95e56d4`.
