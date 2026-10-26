# Idea

## Request

Define and implement the first OWL 2 DL conformance slice through an in-process `rustdl` adapter, with a stable kind-safe Know ID to OWL IRI mapping, standards-correct OWL export, complete model-theoretic query verdicts, and semantic agreement with the Boolean reasoner where their fragments overlap.

## Problem

The previous bridge used declared entities as negative subclass/equivalence witnesses, which misses anonymous OWL models. Disjointness did not establish its negative case. Query-generated probes could describe adapter failures as unsupported user input, the public standards exporter exposed a rustdl lifecycle error, and unchecked n-ary structures could render malformed Functional Syntax.

## Definitions

**First conformance slice:** Named, intersection, union, complement, existential object restriction, and universal object restriction class expressions; declarations; subclass, equivalent-class, and disjoint-class axioms; positive and negative class and object-property assertions; and object-property domain and range axioms.

**Canonical Know IR:** `KnowledgeModule` and typed IDs in `know-ontology` and `know-core`. OWL syntax and rustdl model forms are derived artifacts.

**Complete negative check:** A model-theoretic consistency check of the accepted ontology with the queried proposition added as a temporary OWL constraint. Inconsistency entails the proposition's negation; consistency leaves it open.

**Standards translation:** The public `translate_owl` operation returning `OwlTranslation`: parser-validated Functional Syntax plus standards source-axiom and domain/range metadata. It contains no rustdl syntax or lifecycle state.

**Standards translation error:** A public `OwlTranslationError` from Functional Syntax translation. It identifies unsupported input, invalid OWL structural cardinality, or failed standards-output validation; it is not a rustdl lifecycle error.

**Initialization error:** A `RustdlInitError` produced only while constructing `RustdlReasoner`. It reports adapter input capability or setup failure before querying.

**Unsupported:** Requested input is outside the slice or was rejected before adapter initialization. Generated-probe failures after initialization are not unsupported user input.

**ResourceLimit:** Evaluation stopped before completion because of a backend limit.

**InternalError:** Translation validation, parsing, backend execution, or result decoding failed independently of the logical result.

## Desired outcome

The public translator/exporter emits only parser-validated OWL Functional Syntax and standards metadata for the supported slice, independently of rustdl. The private adapter consumes successful standards translation before building its NNF backend ontology. Inconsistency is reported first. Positive and negative query verdicts are complete: temporary constraints detect anonymous countermodels, open cases remain `Unknown`, and operational failures propagate. Empty and singleton Boolean expressions follow Know semantics, while malformed n-ary class axioms fail atomically.

## Scope

- Strict slice validation, kind-safe IRI translation, and standards Functional Syntax export.
- Rustdl initialization and query adaptation.
- Complete consistency-based negative checks for subclass, equivalence, and disjointness.
- Accurate query failure classification and focused overlap tests.

## Constraints

- `KnowledgeModule` remains canonical.
- OWL 2 Direct Semantics and open-world behavior govern this slice.
- `Inconsistent` precedes ordinary verdicts.
- Empty conjunction is `owl:Thing`, empty disjunction is `owl:Nothing`, and singleton Boolean operations unwrap to their operand.
- Equivalent and disjoint n-ary axioms contain at least two expressions.
- No unsupported or dropped axiom is approximated.
- The public exporter does not return `RustdlInitError`.

## Exclusions

- OWL import into Know IR.
- Rustdl classification integration or direct hierarchy results.
- Diff-reasoner CLI changes.
- Broader OWL constructs, including cardinalities, nominals, data properties, property chains, and characteristics.
- Making rustdl the canonical ontology representation.
