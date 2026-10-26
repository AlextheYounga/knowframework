# Complete Negation and Boundaries Clarification

## Trigger

Review of the approved OWL semantic bridge found that declared-entity enumeration is not a complete proof of negative subclass or equivalence entailment, disjointness only checked one side, generated adapter probes could mislabel backend failures as unsupported input, the public exporter exposed an adapter lifecycle error, and unconstrained n-ary rendering could emit invalid OWL Functional Syntax.

## Decision

Subclass, equivalence, and disjointness queries use OWL model-theoretic consistency probes after inconsistency-first checking. A false positive subclass check is followed by a temporary `SubClassOf` constraint; an inconsistent temporary ontology proves the queried subclass is contradicted. Equivalence first completes both subclass checks, then uses a temporary `EquivalentClasses` constraint unless both directions are entailed. A satisfiable intersection for disjointness is followed by a temporary `DisjointClasses` constraint. A consistent temporary ontology yields `Unknown`; every probe failure propagates as an operational result. This detects anonymous existential witnesses without enumerating declared individuals.

The public `translate_owl` standards translation returns `OwlTranslation`, with Functional Syntax and standards source metadata only, and validates successful output with the OWL parser. `export_owl_functional` delegates only to this translation. `RustdlInitError` remains exclusively the lifecycle error for constructing the rustdl adapter. The private NNF artifact and backend ontology are adapter-owned and are built only after standards translation succeeds.

Know Boolean expression semantics are preserved in OWL: empty conjunction renders as `owl:Thing`, empty disjunction as `owl:Nothing`, and singleton conjunction/disjunction renders as its sole expression. `EquivalentClasses` and `DisjointClasses` require at least two expressions and fail translation atomically with `OwlTranslationError::InvalidStructure`.

After successful adapter initialization, `ReasonError::Conversion` and `RoleChainUnsupported` from generated query probes are operational adapter defects and map to `ReasoningOutcome::InternalError`. Explicit capability failures during initialization remain typed initialization unsupported errors.

## Supersedes

This supersedes the declared-individual counterexample strategy and the claim that query-time conversion errors are unsupported in the previous plan and completion notes. It also supersedes the exporter signature that returned `RustdlInitError` and the lack of structural cardinality validation.

## Required Authoritative Updates

`01-idea.md` records complete temporary-ontology negation checks, standards-level export errors, and the settled n-ary policy. `02-plan.md` assigns these responsibilities and validation. `03-tasks.md` replaces the completed-but-incomplete checks with the corrective implementation and test tasks.
