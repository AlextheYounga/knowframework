# Tasks

## Implementation

- [ ] Define the strict `urn:know:<kind>:<hex>` codec for concept, relation, and
  entity IDs, including canonical decoding and typed error reporting.
- [ ] Introduce an atomic first-slice translation result that records every source
  axiom and relation domain/range rendering, records domain/range coverage
  without inventing support IDs, and rejects unsupported or dropped content before
  rustdl execution.
- [ ] Render the complete first-slice vocabulary: declarations, all six class
  expression forms, subclass/equivalent/disjoint class axioms, positive and
  negative class/object-property assertions, and object-property domain/range;
  encode negative class assertions as complemented `ClassAssertion` and retain
  `NegativeObjectPropertyAssertion` for negative relations.
- [ ] Add typed `RustdlInitError` construction results, returning
  `Unsupported` for unsupported/dropped translation input and `InternalError` for
  parser/backend initialization failures; keep `ReasoningOutcome` for queries.
- [ ] Store canonical source metadata with translated axioms while keeping probe
  axioms distinguishable and outside source support claims.
- [ ] Normalize rustdl capability, completeness, resource-limit, parse, execution,
  and result-decoding statuses to `Unsupported`, `ResourceLimit`, or
  `InternalError` before verdict construction.
- [ ] Implement complete inconsistency-first and proposition/negation dual
  evaluation so `Unknown` is emitted only after both sides complete.
- [ ] Replace synthetic rustdl explanation steps with conservative notes and only
  backend-guaranteed mapped supporting axioms.
- [ ] Keep rustdl classification explicitly unsupported and leave OWL import,
  diff-reasoner CLI, and full OWL 2 DL constructs untouched.

## Validation

- [ ] Add codec cases for round trips, all kind tags, arbitrary UTF-8 IDs,
  malformed hex, noncanonical casing, invalid UTF-8, and cross-kind rejection.
- [ ] Extend relational adapter cases to cover each first-slice construct,
  declaration preservation, domain/range inference, complemented negative class
  assertions, negative object-property assertions, and atomic rejection of
  unsupported/dropped axioms.
- [ ] Verify relation domain/range coverage is recorded while explanations never
  contain invented support IDs for those unannotated items.
- [ ] Add constructor tests for typed `RustdlInitError::Unsupported` and
  `RustdlInitError::InternalError`, distinct from query-time outcomes.
- [ ] Add status fixtures or backend seams proving complete verdicts,
  inconsistency, complete dual-evaluation `Unknown`, `ResourceLimit`,
  `Unsupported`, and `InternalError` remain distinct.
- [ ] Assert explanation support IDs correspond only to translated source axioms
  and that unsupported proof steps are absent.
- [ ] Add the BooleanReasoner/rustdl overlap matrix and make any semantic mismatch
  fail with the proposition and both outcomes visible.
- [ ] Run formatting, linting, compilation, focused adapter tests, overlap tests,
  and the broader repository test suite after implementation; record the results.

## Completion

- [ ] Update `README.md` or `docs/PROJECT.md` if it currently states an OWL
  capability contradicted by the delivered slice.
- [ ] Review the final diff for canonical-IR ownership, no silent axiom drops,
  honest explanations, and the stated exclusions.
- [ ] Mark this task list with implementation validation results and any deliberate
  deviations only after the approved implementation is complete.

## Completion notes

Planning stage: no implementation, test execution, or validation results have
been recorded. The three numbered documents are the authoritative pre-approval
record; implementation requires human review and approval.
