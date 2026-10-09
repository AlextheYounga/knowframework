# Grounding compatibility tasks

## Implementation

- [ ] Add stable core diagnostic codes and messages for omitted and
  incompatible grounding.
- [ ] Change the ontology IR concept grounding field to `Option<Grounding>` and
  preserve the source option through compilation; update compile consumers and
  the omission-preservation test.
- [ ] Implement deterministic grounding-family classification and named-only
  claim selection in `know-admission`, using merged-source lookups while
  iterating only proposal definitions and axioms; emit claim-tied warnings
  when uniquely resolved referenced concepts omit grounding and skip lookups
  invalidated by structural errors.
- [ ] Deduplicate per-concept and claim-tied missing-grounding diagnostics by
  stable semantic identity while preserving proposal traversal order.
- [ ] Replace the empty ontological stage result with the grounding checker and
  extend decision precedence for logical conflicts, ordinary errors, grounding
  deferral, warnings, and clean acceptance.
- [ ] Update admission test fixtures and add focused cases for both families,
  explicit `Primitive`, omitted grounding, proposed-to-existing references,
  claim-tied missing grounding, repeated references, structurally invalid
  lookups, named-only filtering, incompatible claims, and combined-stage
  precedence.
- [ ] Update user-facing admission documentation if the repository's current
  description does not state the new grounding outcomes.

## Validation

- [ ] Run the focused ontology compile tests and verify omitted grounding is
  retained as `None` rather than converted to `Primitive`.
- [ ] Run admission tests and verify each settled decision and diagnostic code,
  including claim-tied missing-grounding warnings, deterministic deduplication,
  skipped diagnostics for invalid lookups, and non-conflict classification for
  grounding incompatibility.
- [ ] Run the broader workspace semantic test suite, formatting/static checks,
  and inspect the stage audit records to confirm all five stages remain
  represented.
- [ ] Review the final diff for deterministic ordering, no grounding
  inheritance/inference, no checks on excluded expression forms, and no
  grounding diagnostics invented for structurally invalid symbols.

## Completion

- [ ] Remove obsolete vacuous-stage comments and stale grounding TODO text.
- [ ] Confirm only intended implementation, test, and documentation files are
  changed before the feature commit.

## Completion notes

Implementation has not started. Validation results and deviations will be
recorded here after the approved implementation is complete.
