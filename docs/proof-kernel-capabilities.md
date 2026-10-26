# Proof Kernel Capability Map

This map reconciles the proof-kernel direction in [`idea.md`](idea.md) with the
executable system. It distinguishes the canonical calculus from ontology
reasoners that can remain optimized executors for supported fragments.

## Implemented and reusable

| Area | Existing implementation | Role in the proof-kernel architecture |
| --- | --- | --- |
| Stable knowledge identity and provenance | `know-core` | Reuse for named declarations and source lineage. These IDs do not constitute logical equality. |
| Ontology source and validated IR | `know-ontology` | Reuse as the current description-logic source model and as an eventual compilation input. It is not the canonical proof language. |
| Boolean model reasoning | `know-reasoner` | Retain as an optimized decision procedure for its complete fragment. Its explanations are not yet kernel-checkable proof certificates. |
| OWL translation and relational reasoning | `know-owl` | Retain as a standards bridge and optimized executor where its semantics match compiled kernel theories. Backend completion does not define proof validity. |
| Proposal and regression gates | `know-admission` | Reuse the admission workflow after checked theories and theorem regressions can participate in it. |
| Typed terms and universes | `know-kernel` | `Prop`, predicative universes, dependent function types, lambdas, application, and named constants are executable. |
| Binding and substitution | `know-kernel` | De Bruijn indices provide alpha-invariant binding, scope checking, shifting, and capture-avoiding substitution. |
| Proof checking | `know-kernel` | A proof is a term checked against a proposition. Checking is independent of proof search and external reasoners. |
| Equality | `know-kernel` | Equality formation, reflexivity, and substitution through a proposition-valued motive are checked directly. |
| Definitions | `know-kernel` | Transparent non-recursive definitions are type checked and unfolded for definitional equality. |
| Assumption lineage | `know-kernel` | Checked theorems report their transitive named logical assumptions. Symbols and pure definitions are not reported as premises. |

An opaque term whose type is a proposition cannot be introduced as an ordinary
symbol; it must be an explicit assumption. This prevents declarations from
bypassing theorem dependency reporting.

The executable acceptance cases are in
`crates/know-kernel/tests/foundation.rs`. They establish:

- a universally required characteristic applies to a declared being;
- identity permits substitution in predication;
- a declared transitivity premise licenses the corresponding consequence;
- a proposition and its negation produce `False`;
- a universal classification does not itself prove an existential claim;
- implication and universal proofs are checked lambda terms;
- conjunction is definable and checkable in the ordinary calculus rather than
  requiring a special kernel rule;
- excluded middle is an explicit assumption rather than a hidden kernel rule;
- wrong argument types, escaped variables, circular definitions, and
  unsupported conclusions are rejected.

## Implemented with different semantics

| Capability | Current behavior | Required relationship to the kernel |
| --- | --- | --- |
| Concepts | Unary class expressions interpreted by ontology reasoners | Elaborate to proposition-valued predicates over a declared domain. |
| Relations | Named binary relations with optional ontology domain and range | Elaborate to typed curried functions ending in `Prop`. |
| `Exists` and `ForAll` in ontology expressions | Description-logic role restrictions | Keep distinct from general dependent universal and existential quantification. Similar spelling must not collapse their meanings. |
| Ontology axioms | Fixed variants such as subclass, disjointness, and assertions | Compile supported forms into propositions and checked assumptions or theorems. |
| Reasoner explanations | Backend inference steps and supporting ontology axioms | Convert to checked proof terms or identify the backend as an explicit trusted dependency. |

## Specified but not implemented

- A parser and source spans for the readable calculus.
- Named source binders elaborated to de Bruijn indices.
- Context declarations, imports, and conflict handling.
- Inductive data types and their checked eliminators.
- Named standard-library declarations and eliminators for conjunction,
  disjunction, and existential quantification. Conjunction is already exercised
  through an ordinary calculus definition; no stable source-level library exists yet.
- Terminating recursive definitions and a structural termination checker.
- Stratified quotation, claims, evidence, and revisions.
- Theorem queries and proof search outside the kernel.
- Compilation between ontology modules and kernel theories.
- Kernel-checkable certificates from the Boolean and OWL executors.
- Persistent serialization and canonical hashing of checked theories.

## Decisions required before the next semantic extensions

These decisions concern exact language semantics, not whether the requested
behavior should be implemented:

1. Select the concrete surface grammar and diagnostic span model while keeping
   the current kernel term representation independent of syntax.
2. Specify the initial inductive core, including whether logical connectives
   are ordinary inductive declarations or a smaller primitive set from which
   the standard library derives them.
3. Specify universe constraints for user declarations beyond the current
   `Prop` and predicative `Universe(level)` hierarchy.
4. Define the certificate or explicit trust boundary used when existing
   reasoners accelerate kernel queries.
5. Define how ontology open-world verdicts map to proof obligations without
   treating failed proof search as proof of negation.

## Boundary

`know-kernel` is the trusted proof checker. It does not interpret natural
language, discover proofs, decide whether assumptions are true, or silently
promote ontology reasoner output into proofs. Those systems may propose terms;
the kernel accepts only terms that satisfy its typing rules.
