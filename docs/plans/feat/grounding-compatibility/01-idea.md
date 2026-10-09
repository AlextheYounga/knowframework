# Grounding compatibility admission

## Request

Replace admission stage 3's vacuous grounding check with deterministic
accepted, deferred, and rejected behavior. Grounding compatibility must protect
identity and subsumption claims without turning omitted metadata into an
asserted `Primitive` grounding.

## Problem

The admission pipeline currently records that the ontological stage ran but
always passes it. In addition, compilation maps an omitted source grounding to
`Grounding::Primitive`, so admission cannot distinguish genuinely unknown
grounding from an explicit unconstrained category. This permits incompatible
identity or subsumption claims to reach later stages and gives no deterministic
deferral for proposals whose grounding is missing.

## Definitions

**Omitted grounding:** A proposed concept record whose source `grounding` is
`None`. It is unknown, not equivalent to `Grounding::Primitive`.

**Primitive grounding:** An explicit `Grounding::Primitive` value. It is a
deliberate unconstrained category and is compatible with either grounding
family.

**Occurrent family:** `Process` and `Event`.

**Continuant/classificatory family:** `NaturalKind`, `StructuralDefinition`,
`FunctionalKind`, `SocialKind`, `LegalKind`, `Role`, `MeasurementClass`, and
`EngineeredConcept`.

**Grounding compatibility claim:** A proposal assertion that directly claims
identity or subsumption between named concepts: a proposed concept definition
whose expression is one `Named` concept; a `SubclassOf` axiom with named child
and parent; or an `EquivalentClasses` axiom in which every expression is named.
Complex Boolean or restriction expressions and disjointness are not grounding
compatibility claims.

**Merged source:** The base `KnowledgeModuleSource` plus all proposal records.
Grounding lookup uses this source so a proposed named concept can be checked
against an existing concept.

**Grounding-required warning:** A warning attached to a specific
grounding-compatibility claim when one of the named concepts required by that
claim has omitted source grounding. This is distinct from the per-concept
warning for a newly proposed concept whose own grounding is omitted.

## Desired outcome

Stage 3 emits deterministic diagnostics and the pipeline makes these decisions:

- Missing grounding on any newly proposed concept emits a warning and produces
  `DeferredForGrounding` when no error stage changes the outcome.
- A proposal-originated named identity/subsumption claim whose base or
  proposed referenced concept has omitted grounding emits a claim-tied
  grounding-required warning and produces `DeferredForGrounding` when no error
  stage changes the outcome.
- Missing-grounding diagnostics are deduplicated deterministically: one
  per-concept warning per missing newly proposed concept and one claim-tied
  warning per distinct missing concept/claim, without noisy repeats.
- If structural validation makes a named lookup missing or ambiguous, stage 3
  emits no grounding diagnostic for that lookup; structural error precedence
  determines the decision.
- Explicit `Primitive` is never treated as missing and is compatible with both
  families.
- Claims linking two non-neutral concepts from different families emit a
  grounding incompatibility error and produce `Rejected` when no logical
  conflict takes precedence.
- Claims within one family and claims involving `Primitive` pass this gate.
- Complex Boolean/restriction expressions and disjointness do not produce
  grounding compatibility diagnostics.

## Scope

This change covers stage-3 policy, grounding-family classification, source/IR
preservation of omitted grounding, decision precedence, diagnostics, focused
admission tests, and documentation of the resulting behavior.

## Constraints

Grounding is checked only on proposal-originated definitions and axioms in the
specified claim forms. Grounding is neither inherited nor synthesized in the
source or IR. An incompatible grounding claim is an error, never a deferral or
an ontological conflict classification. A claim-tied missing-grounding warning
is emitted only when the symbol lookup is unique and structurally valid.
Missing-grounding warnings defer only when all applicable stages are free of
errors.

Decision precedence is explicit: a logical inconsistency marked as a conflict
produces `ConflictsWithExistingKnowledge`; otherwise any error from structural,
lexical, ontological, logical, or regression validation produces `Rejected`;
otherwise grounding-missing warnings produce `DeferredForGrounding`; other
warnings produce `AcceptedWithWarnings`; no diagnostics produce `Accepted`.

## Exclusions

This change does not redesign the grounding taxonomy, encode grounding in OWL,
persist or arbitrate proposals, infer grounding, inherit grounding through
taxonomy, or alter logical treatment of Boolean expressions, restrictions, or
disjointness.
