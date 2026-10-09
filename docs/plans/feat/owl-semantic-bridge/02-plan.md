# Plan

## Current behavior

- `know-ontology::KnowledgeModule` is the validated canonical IR. `ConceptExpr`
  already contains the six requested expression forms, while `Axiom` contains
  the requested class and object-property assertions and class axioms. Relation
  records carry domain and range expressions; declarations are emitted from
  module records (`crates/know-ontology/src/ir.rs`).
- `know-reasoner::Reasoner` exposes `Proposition`, `Verdict`, and
  `ReasoningOutcome`, including `Unsupported`, `ResourceLimit`, and
  `InternalError` (`crates/know-reasoner/src/lib.rs`). BooleanReasoner is complete
  for the role-free Boolean fragment and rejects relational input as unsupported.
- `know-owl::RustdlReasoner` currently exports Functional Syntax, parses it, and
  calls rustdl consistency, satisfiability, and instance/subclass probes
  (`crates/know-owl/src/lib.rs`). Its hex IRI encoding is one-way, translation has
  no capability ledger, `backend_error` conflates backend failures with
  unsupported behavior, and explanations contain a synthetic BooleanRefutation
  step with no source support. Its current `NegativeClassAssertion` renderer also
  emits a non-existent OWL Functional Syntax construct; OWL represents this case
  as a class assertion whose expression is `ObjectComplementOf`.
- Existing `know-owl/tests/relational.rs` covers asserted relations and
  domain/range membership, but not mapping round trips, dropped-axiom handling,
  complete dual evaluation, failure classes, or Boolean overlap. The CLI and
  admission pipeline consume the shared reasoner interfaces and are outside this
  slice's integration scope.

## Intended behavior

1. A translation pass validates every expression, declaration, relation record,
   and annotated axiom against the first conformance slice. It produces an OWL
   ontology plus source-to-rendered-axiom metadata. Any unsupported or dropped
   source item stops initialization with `RustdlInitError::Unsupported`; no
   partial ontology is reasoned over. OWL Functional Syntax has no
   `NegativeClassAssertion`, so a Know negative class assertion renders as
   `ClassAssertion(ObjectComplementOf(<class expression>) <individual>)`.
   Negative object-property assertions render as
   `NegativeObjectPropertyAssertion`.
2. The ID codec emits `urn:know:<kind>:<lowercase UTF-8 hex>` for the three typed
   ID spaces. Decoding validates the scheme, exact kind, canonical lowercase
   encoding, even byte count, and UTF-8. A decoded ID must have the same kind and
   exact string value as its input; malformed or cross-kind IRIs are adapter
   errors, not alternate IDs.
3. The adapter keeps source `AxiomId` metadata alongside each translated axiom.
   Relation-record domain and range items are included in the coverage/capability
   ledger but have no source `AxiomId`, so they never receive invented support
   IDs. Translation/parser/backend setup failures are returned from construction
   as `RustdlInitError::InternalError` when they are not unsupported input;
   callers preserve this distinction. Query-time backend results are converted
   through one outcome-normalization boundary:
   unsupported/dropped capabilities become `Unsupported`; incomplete or timed-out
   work becomes `ResourceLimit`; parse, execution, and result-decoding failures
   become `InternalError`.
4. Query evaluation first handles complete ontology inconsistency. For ordinary
   propositions, positive and negative entailment are evaluated as a tracked pair.
   `Entailed` or `Contradicted` is emitted only for a complete corresponding
   result; `Unknown` requires both complete negative answers. Any incomplete side
   propagates `ResourceLimit`, never `Unknown` or `Contradicted`.
5. Explanations use only backend-reported mapped source axioms. When rustdl gives
   no proof/support artifact, the explanation has no inference steps and says
   only that the backend completed the stated entailment or consistency check; it
   does not claim a fabricated refutation or witness.
6. A focused overlap matrix compares rustdl with BooleanReasoner for supported
   Boolean modules and propositions, including positive/negative class assertions,
   subclass/equivalence/disjointness, satisfiability, consistency, and the
   expected unsupported boundary for relational constructs. Differences fail
   validation rather than being normalized away.

## Approach

Refactor `know-owl` into focused translation, mapping, outcome, and explanation
responsibilities while retaining `RustdlReasoner` as the `Reasoner` implementation.
The constructor performs the strict translation/capability pass once and returns
`Result<Self, RustdlInitError>`. `RustdlInitError::Unsupported` represents
unsupported or dropped translation input; `RustdlInitError::InternalError`
represents parser or backend initialization failure. A successful constructor
stores the immutable ontology, source-axiom map, coverage ledger, and typed
vocabulary. Query methods use small probe ontologies only for the requested
proposition, with probe entities and classes kept outside the source support map.
A single normalization layer converts rustdl's supported/incomplete/error signals
before query code constructs a `ReasoningOutcome`.

The adapter will continue to expose `classify` as unsupported, will not add import
or CLI integration, and will retain `KnowledgeModule` as the only canonical input.

## Responsibilities and boundaries

- `know-core`: owns typed ID values and stable diagnostic primitives; it does not
  know OWL IRI syntax.
- `know-ontology`: owns `ConceptExpr`, `Axiom`, relation domain/range records,
  annotations, and validation of the canonical module.
- `know-owl` mapping boundary: owns the strict kind-tagged IRI codec and rejects
  malformed/cross-kind values.
- `know-owl` translation boundary: owns slice capability validation, Functional
  Syntax rendering, declarations, domain/range rendering, and source axiom
  metadata. It records domain/range coverage without assigning source support IDs
  and must fail closed on dropped content.
- `know-owl` outcome boundary: owns rustdl status classification, complete dual
  evaluation, and conversion to `ReasoningOutcome`.
- `know-owl` explanation boundary: owns conservative support and note creation;
  it never invents proof steps.
- `know-reasoner`: remains the shared verdict contract and Boolean reference
  backend; its semantics are not weakened to match rustdl.
- `know-owl` tests: own adapter conformance and Boolean-overlap fixtures. CLI,
  admission, and import behavior remain consumers or exclusions.

## Affected areas

- `crates/know-owl/src/lib.rs`: expose the refactored reasoner and translation
  entry points and typed `RustdlInitError` while preserving public API intent.
- `crates/know-owl/src/iri.rs`, `translate.rs`, `outcome.rs`, and `explanation.rs`:
  focused new modules for the codec, slice adapter contract, status mapping, and
  honest explanations.
- `crates/know-owl/tests/relational.rs`: extend existing slice coverage for
  declarations, class axioms, assertions, domain/range, and failure semantics.
- `crates/know-owl/tests/overlap.rs`: add the BooleanReasoner/rustdl overlap
  matrix.
- `crates/know-reasoner/src/lib.rs`: update only shared result metadata or
  inference-rule vocabulary required to represent backend-honest explanations;
  no backend-specific types belong here.
- `docs/PROJECT.md` or `README.md`: document the delivered slice and explicit
  unsupported boundaries if the existing project documentation describes the
  current OWL capability.

## Decisions

- Use the existing `urn:know` namespace with a typed segment and UTF-8 hex payload.
  It is IRI-safe, deterministic, reversible for arbitrary ID strings, and keeps
  concept, relation, and entity namespaces disjoint without changing Know IDs.
- Treat translation as atomic. A single unsupported or dropped source axiom
  invalidates construction with `RustdlInitError::Unsupported` instead of
  silently producing a weaker ontology. Parser or backend setup failures return
  `RustdlInitError::InternalError`; `ReasoningOutcome` is reserved for queries
  after successful construction.
- Keep source axiom IDs separate from OWL entity IRIs. Entity mapping identifies
  concepts, relations, and individuals; support metadata identifies Know axioms.
- Render Know negative class assertions as complemented `ClassAssertion` axioms,
  because `NegativeClassAssertion` is not OWL 2 Functional Syntax. Retain
  `NegativeObjectPropertyAssertion` for negative relations.
- Track relation domain/range records in translation coverage and capability
  metadata, but never synthesize `AxiomId` values for those unannotated items.
- Normalize statuses before verdict construction. Backend error text is not
  parsed heuristically into `Unsupported`; only an explicit capability/drop
  result is unsupported, operational limits are resource limits, and all other
  backend/parse failures are internal errors.
- Require complete dual evaluation for `Unknown`. This preserves OWL's open-world
  semantics while preventing one failed negative probe from masquerading as an
  open result.
- Prefer no explanation steps over unsupported claims. A backend result without a
  guaranteed proof artifact receives an accurate note and only guaranteed mapped
  supports.
- Keep rustdl classification unsupported in this slice; hierarchy integration is a
  separate semantic and API decision.

## Risks

- Functional Syntax rendering can omit relation-record domain/range or an
  annotation unless every source item is tracked; the capability ledger and
  translation tests must assert one-to-one coverage.
- A codec that accepts noncanonical or cross-kind IRIs can alias typed IDs; strict
  decode tests must cover malformed hex, invalid UTF-8, casing, and wrong kinds.
- Rustdl may provide positive entailment without complete negative evaluation or
  a justification. The outcome and explanation boundaries must preserve that
  limitation rather than infer `Unknown` or fabricate proof data.
- Existing callers may assume all rustdl query failures are `Unsupported`; tests
  and documentation must assert the new `ResourceLimit` and `InternalError`
  distinctions without changing unrelated BooleanReasoner behavior.
- Inconsistency must remain first-class and precede ordinary query evaluation;
  otherwise probe-based contradiction checks can obscure the ontology-wide result.

## Validation

- Codec tests prove all three ID kinds round-trip, remain kind-safe, and reject
  malformed, noncanonical, and cross-kind IRIs.
- Translation tests prove every first-slice expression and axiom renders, every
-  module declaration is present, domain/range inference remains available,
  negative class assertions render as complemented `ClassAssertion`, negative
  object-property assertions render as `NegativeObjectPropertyAssertion`, and a
  deliberately unsupported/dropped item returns typed initialization
  `Unsupported` without execution.
- Initialization tests prove parser/backend setup failures return typed
  initialization `InternalError`, while query-time failures retain their
  `ReasoningOutcome` classification.
- Outcome tests use controlled backend statuses to prove complete entailment,
  contradiction, inconsistency, complete `Unknown`, `ResourceLimit`,
  `Unsupported`, and `InternalError` are distinct.
- Explanation tests prove support IDs are only mapped source axioms and that no
  synthetic inference step or unsupported backend claim is emitted; domain/range
  coverage is present without fabricated support IDs.
- The overlap matrix compares BooleanReasoner and rustdl on matching complete
  cases and records the relational unsupported boundary.
- Run the repository's formatting, lint, compile, focused tests, broader tests,
  and final diff review after implementation; this planning change itself is
  limited to documentation and is not test-executed.
