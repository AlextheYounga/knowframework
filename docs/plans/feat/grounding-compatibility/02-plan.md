# Grounding compatibility implementation plan

## Current behavior

`know-admission::Pipeline::admit` merges the base and proposal source, compiles
the merge, runs structural and lexical stages, then inserts an empty
ontological result at stage 3. Logical and regression stages operate on the
compiled module when available. `decide` currently classifies a logical
conflict first, then any error as `Rejected`, then warnings as
`AcceptedWithWarnings`, and otherwise `Accepted`; it has no grounding deferral
path.

`ConceptRecordSource::grounding` is optional, but
`know-ontology::compile::resolve_concept` currently converts `None` to
`Grounding::Primitive` in `ir::Concept`. The source axiom forms and concept
expressions provide the exact named-only shapes needed for the new check.

## Intended behavior

Stage 3 runs after structural and lexical validation using the merged source.
It reports one warning for each newly proposed concept with omitted grounding.
It examines only proposal-originated grounding compatibility claims:

1. A proposed concept with `definition: Some(Named(target))` compares the
   proposed concept with `target`.
2. A proposed `SubclassOf` compares named child and parent.
3. A proposed `EquivalentClasses` compares its named expressions pairwise when
   all expressions are named.

Grounding lookup reads both proposal and base concepts from the merged source.
An omitted lookup value is unknown for compatibility and does not become a
neutral value. When a named claim's uniquely resolved child, parent, definition
target, or equivalent member exists but has omitted source grounding, the stage
emits a grounding-required warning tied to that claim. A claim is incompatible
only when both values are present, both are non-`Primitive`, and their families
differ. Same-family claims and any claim involving explicit `Primitive` pass.
Claims with complex expressions or disjointness are ignored by this stage. If
structural validation means a named symbol is absent or ambiguous, the checker
skips that lookup and emits no grounding diagnostic.

The checker deduplicates diagnostics by stable semantic keys and emits them in
proposal order: each newly proposed concept produces at most one
missing-grounding warning, and each distinct claim/missing-concept pair
produces at most one grounding-required warning. Repeated references therefore
remain auditable without producing noisy repeats.

The stage records errors for incompatible claims and warnings for omitted
grounding. The decision function recognizes grounding-missing diagnostics and
returns `DeferredForGrounding` only after confirming there is no error. Logical
inconsistency retains conflict classification precedence; all other stage
errors, including grounding incompatibility, return `Rejected`. Thus an
incompatible grounding claim itself never sets the logical conflict flag and
does not become `ConflictsWithExistingKnowledge`. If there are no errors,
grounding deferral precedes generic warning acceptance. Stage execution and
audit results remain deterministic.

## Approach

Preserve the existing five-stage pipeline and add a focused source-level
grounding checker in `know-admission`. Define small explicit helpers for
grounding family classification, named-expression extraction, merged-source
lookup, and diagnostic collection. Pass the proposal and merged source to the
checker without adding grounding to records or axioms.

Change the ontology IR concept field to retain `Option<Grounding>` and pass the
source option through compilation. Update only consumers and fixtures/tests
that require the type change. Add stable core diagnostic codes for missing and
incompatible grounding. Extend admission tests for every family, neutral
Primitive, omitted grounding, claim-tied missing grounding for proposed and
existing concepts, deterministic deduplication, structurally invalid lookups,
named-only claim filtering, incompatible rejection, and combined-stage
precedence.

## Responsibilities and boundaries

- `know-ontology::source` remains the serialized declaration of optional
  grounding.
- `know-ontology::compile` preserves the option instead of defaulting it.
- `know-ontology::ir` represents omitted grounding as `Option<Grounding>`.
- `know-admission` owns claim selection, family compatibility, diagnostics,
  and final decision precedence.
- `know-core::codes` owns stable diagnostic identifiers.
- `crates/know-admission/tests/admission_tests.rs` owns observable pipeline
  behavior tests; ontology compile tests cover preservation of omission.
- README/project documentation is updated only where current admission behavior
  is described; no knowledge corpus changes are required.

## Affected areas

- `crates/know-admission/src/lib.rs`
- `crates/know-admission/tests/admission_tests.rs`
- `crates/know-ontology/src/compile.rs`
- `crates/know-ontology/src/ir.rs`
- `crates/know-ontology/tests/compile_tests.rs`
- `crates/know-core/src/lib.rs`
- `README.md` and/or relevant project documentation if the admission contract
  requires a user-facing update

## Decisions

- Use two compatibility families exactly as defined in `01-idea.md`; do not
  introduce an ordering or inferred subtype relation among family members.
- Treat explicit `Primitive` as neutral, not as omitted data.
- Preserve omission in the IR so the admission boundary can distinguish
  unknown from explicit neutral grounding.
- Check proposal claims against the merged source, but do not revalidate
  unrelated pre-existing axioms as newly proposed claims.
- Emit claim-tied missing-grounding warnings only for unique symbols available
  in the structurally valid merged source; structural failures own missing or
  ambiguous symbol diagnostics.
- Restrict checks to direct identity/subsumption-shaped assertions. Logical
  validation remains responsible for complex expressions and disjointness.
- Keep logical conflict detection separate from grounding incompatibility so
  grounding errors cannot be mislabeled as conflicts with existing knowledge.
- Emit per-concept missing-grounding warnings and per-claim incompatibility
  errors with stable diagnostic codes and deterministic traversal order.
- Deduplicate per-concept and claim-tied missing-grounding diagnostics by
  semantic identity before returning the stage result.

## Risks

- Changing the IR field from `Grounding` to `Option<Grounding>` can expose
  assumptions in downstream code; compile-time consumers and tests must be
  updated without introducing a default.
- Existing admission tests that build concepts without groundings will change
  from clean acceptance to grounding deferral; tests must make intentional
  groundings explicit where acceptance is the subject under test.
- Checking the merged source can accidentally inspect base-only claims; the
  implementation must iterate proposal definitions and proposal axioms only.
- A missing or ambiguous symbol after structural failure can create duplicate
  or misleading grounding reports; the checker must distinguish a unique
  omitted-grounding record from an unavailable symbol and skip the latter.
- A broad expression traversal could over-report compatibility for logical
  structure; only the named-only forms in the settled policy may emit
  grounding diagnostics.
- Combining diagnostics from several stages can obscure the final decision if
  precedence is not centralized; `decide` must remain the single classifier.

## Validation

The implementation will be validated with focused admission tests that prove:

- every occurrent and continuant/classificatory pair within its family passes;
- explicit `Primitive` passes against each family and does not defer;
- omitted grounding on every newly proposed concept warns and defers;
- a proposed definition can compare to an existing named concept;
- a named claim referencing an omitted-grounding proposed or existing concept
  emits one claim-tied warning and defers;
- repeated references produce deterministic, deduplicated diagnostics;
- named `SubclassOf` and all-named `EquivalentClasses` are checked;
- Boolean, restriction, and disjointness forms produce no grounding diagnostic;
- structurally missing or ambiguous symbols produce no invented grounding
  diagnostic;
- cross-family claims are rejected with the incompatibility code and are not
  conflicts;
- structural, lexical, logical-conflict, logical-error, and regression errors
  follow the stated precedence when grounding warnings or errors coexist.

Ontology compile tests will prove omitted grounding remains `None` in the IR.
The normal admission and ontology test suites, formatting/static checks, and a
final diff review will verify no vacuous stage behavior, accidental grounding
inference, or unrelated file changes remain.
