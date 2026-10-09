# Idea

## Request

Define and implement the first OWL 2 DL conformance slice through the in-process
`rustdl` backend. The slice must preserve a stable, reversible, kind-safe Know ID
to OWL IRI mapping, specify an explicit Know IR to `rustdl` adapter contract,
preserve the distinctions between complete verdicts and operational failures, and
make BooleanReasoner and `rustdl` agree on their semantic overlap.

## Problem

`know-owl` currently renders a useful subset of `KnowledgeModule` values, but the
boundary is implicit. Its IRI encoder has no inverse or kind validation, translation
does not report dropped or unsupported input, backend errors are partly classified
as `Unsupported`, and the adapter constructs explanation steps and empty support
sets without a backend guarantee. The current query flow can also return `Unknown`
after only a one-sided or incomplete evaluation. These behaviors can silently
weaken OWL semantics and make results incomparable with the complete Boolean
backend.

## Definitions

**First conformance slice:** The supported OWL-aligned fragment consisting of
named, intersection, union, complement, existential object restriction, and
universal object restriction class expressions; concept, object-property, and
individual declarations; subclass, equivalent-class, and disjoint-class axioms;
positive and negative class and object-property assertions; and object-property
domain and range axioms. It excludes every construct not listed here.

**Canonical Know IR:** `KnowledgeModule` and its typed IDs in `know-ontology` and
`know-core`. Rustdl model types and OWL Functional Syntax are adapter artifacts,
never the source of truth.

**Complete dual evaluation:** A query evaluation in which both the proposition and
its logical negation have been evaluated to completion, or a complete consistency
result has established the applicable entailment. `Unknown` is valid only when
complete evaluation establishes that neither side follows.

**Unsupported:** The requested target semantics or an input axiom is outside this
slice, or the adapter/backend reports that an axiom was dropped. Unsupported input
is never approximated by omitting it.

**ResourceLimit:** Evaluation stopped before completion because of a time,
iteration, memory, or equivalent backend limit. In particular, an incomplete
negative evaluation is not `Unknown` or `Contradicted`.

**InternalError:** Translation, OWL parsing, backend execution, or result decoding
failed independently of the logical result.

**Mapped supporting axiom:** A source `AxiomId` whose corresponding translated OWL
axiom was actually used or explicitly reported by the backend as support. The
adapter must not invent proof steps, supporting axioms, or causal explanations.
Relation-record domain and range items have no source `AxiomId`; they are tracked
for translation coverage and capability validation but never receive invented
support IDs.

**Initialization error:** A typed failure from `RustdlReasoner::new` or its
translation/parsing setup. `RustdlInitError::Unsupported` means the input uses an
unsupported or dropped construct; `RustdlInitError::InternalError` means parsing,
backend initialization, or another setup failure independent of logical meaning.
Initialization errors are not query-time `ReasoningOutcome` values.

## Desired outcome

The adapter accepts exactly the first conformance slice, translates it without
semantic loss, and rejects unsupported or dropped content before returning a
logical result. Every mapped Know ID round-trips through its OWL IRI and cannot be
decoded as a different ID kind. Queries return `Complete` verdicts only for
completed backend evaluations; incomplete evaluations return `ResourceLimit`,
backend/parse failures return `InternalError`, and `Unknown` is returned only after
complete dual evaluation. Explanations state only backend-guaranteed facts and
mapped supports. BooleanReasoner and rustdl agree on their shared Boolean cases.

## Scope

- Define and enforce the first conformance slice at the Know/rustdl boundary.
- Define a canonical, reversible, kind-safe IRI codec for concepts, relations, and
  entities.
- Translate declarations, class definitions, listed class axioms, assertions, and
  relation domain/range into OWL 2 Functional Syntax and the rustdl model.
- Encode Know negative class assertions as
  `ClassAssertion(ObjectComplementOf(<class expression>) <individual>)`; use
  `NegativeObjectPropertyAssertion` for negative object-property assertions.
- Track every source axiom through translation and reject unsupported/dropped
  axioms rather than approximating them.
- Normalize rustdl capability, completeness, timeout, execution, and parsing
  outcomes into `ReasoningOutcome` and `Verdict`.
- Preserve honest explanations and verify semantic agreement with BooleanReasoner
  on the overlap.

## Constraints

- `KnowledgeModule` remains canonical; rustdl types do not leak into Know IR.
- OWL 2 Direct Semantics and open-world behavior govern the supported slice.
- `Inconsistent` is reported before ordinary query verdicts when the ontology has
  no model.
- Complete positive and negative assertions, domain/range inferences, and
  expression entailments must not be replaced with closed-world assumptions.
- No unsupported or dropped axiom may influence a result through approximation.
- Constructor and translation setup failures use typed initialization errors;
  query-time failures use `ReasoningOutcome`.
- Explanations may contain only backend-guaranteed mapped supports; fabricated
  inference steps are prohibited.
- Planning is the current work product; implementation and test execution happen
  only after human approval of this record.

## Exclusions

- OWL import into Know IR.
- Rustdl classification integration or direct hierarchy results.
- Diff-reasoner CLI changes.
- Full OWL 2 DL, including cardinalities, nominals, data properties, property
  chains and characteristics not listed in the slice.
- Making rustdl the canonical ontology representation.
