# Tasks

## Implementation

- [ ] Extend and reuse the semantic bridge's shared `urn:know`/external IRI
  codec from both OWL import and export, including strict canonical and
  typed-kind validation; do not add competing identity logic.
- [ ] Add Functional Syntax subset validation that rejects every excluded
  construct, malformed/relative identity, and unsupported ontology metadata
  before producing source records.
- [ ] Convert supported declarations, expressions, domain/range axioms,
  assertions, subclass/equivalent/disjoint axioms into Know source records;
  recognize `ClassAssertion(ObjectComplementOf(C) i)` as negative class
  membership and emit it in that standard form.
- [ ] Normalize identities and records, infer only unique equivalent-class
  definitions while consuming their equivalent statements, assign deterministic
  labels/provenance/module IDs, sort all vectors and axioms, and compute the
  always-prefixed module ID hash from ontology IRI bytes or canonical content.
- [ ] Update OWL export to re-emit external absolute IDs verbatim while keeping
  non-IRI Know IDs in canonical Know URNs.
- [ ] Implement the CLI `ImportOwl` path read, import delegation, canonical RON
  serialization, stdout behavior, and nonzero error path.
- [ ] Add focused OWL tests for each supported construct, identity edge case,
  rejection case, deterministic normalization, and import/export round trip.
- [ ] Update README/current-capability documentation with the supported subset
  and explicit unsupported-input behavior.

## Validation

- [ ] Confirm supported Functional Syntax imports into schema-version-1 source
  that compiles without unresolved IDs and preserves every supported semantic
  statement exactly once, including complement-shaped negative class membership.
- [ ] Confirm bridge-codec integration tests prove lossless canonical Know IDs,
  exact external IRI preservation, malformed-URN rejection, and cross-kind
  rejection.
- [ ] Confirm excluded constructs fail explicitly and no partial records are
  returned.
- [ ] Confirm reordered equivalent inputs produce identical module IDs,
  canonical RON, axiom IDs, and exports, and confirm module hashes use ontology
  IRI UTF-8 bytes when present and canonical content otherwise.
- [ ] Confirm CLI success output parses as canonical RON and CLI failures return
  nonzero with a construct-specific message.
- [ ] Confirm a uniquely defining equivalent-class statement is consumed once
  and other equivalent statements remain exactly once through export.
- [ ] Run focused tests, workspace tests excluding cleancode as appropriate,
  `cargo fmt --all --check`, clippy, and final diff review.

## Completion

- [ ] Remove obsolete import placeholder/error text and stale capability claims.
- [ ] Verify no supported input is silently dropped and no unrelated files or
  generated artifacts are included.
- [ ] Record deviations, validation evidence, and deliberately unfinished OWL
  features in the completion notes.

## Completion notes

Not started. This planning record intentionally contains no implementation or
test results.
