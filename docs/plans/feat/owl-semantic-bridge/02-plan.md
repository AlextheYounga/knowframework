# Plan

## Current behavior

`KnowledgeModule` is the canonical validated IR; `ConceptExpr` supports Boolean and object-restriction expressions and `Axiom` carries the slice's class and relation axioms. `know-owl` has strict IRI mapping, public standards translation, private rustdl NNF adaptation, and a rustdl reasoner. The prior query code attempted negative subclass/equivalence proofs by scanning declared entities, while OWL can require anonymous witnesses.

## Intended behavior

A validated module exports parser-valid OWL 2 Functional Syntax through a standards-level API. `RustdlReasoner::new` separately translates its private backend artifact and reports lifecycle failures. Query evaluation remains inconsistency-first. False subclass/equivalence/disjoint positive checks are resolved by adding the standards-equivalent requested constraint to a temporary ontology and completing consistency evaluation. An inconsistent temporary ontology produces `Contradicted`; a consistent one produces `Unknown`; all operational outcomes propagate.

## Approach

Keep focused standards translation, rustdl adaptation, IRI, outcome, and query modules. Public `translate_owl` returns `OwlTranslation`, owning Functional Syntax rendering, source metadata, structural validation, Boolean normalization, and standards parser validation. Private rustdl adaptation consumes only a successful standards translation and the validated module to render NNF and build its backend ontology. `lib` maps translation errors to `RustdlInitError` only in `RustdlReasoner::new`: rejected input is `Unsupported`; invalid generated output is `InternalError`.

The query layer adds small temporary OWL constraints to the immutable backend ontology: `SubClassOf(child parent)`, `EquivalentClasses(left right)`, and `DisjointClasses(left right)`. It never relies on enumerating source entities as a proof of negation. Equivalence completes both positive directions before its consistency probe. The outcome layer treats all post-initialization conversion and role-chain errors as `InternalError`.

## Responsibilities and boundaries

- `translate.rs` owns the public standards `OwlTranslation`, its source metadata, standard render/parse validation, and structural policy.
- `backend.rs` owns private rustdl NNF rendering and backend ontology construction.
- `init.rs` owns `RustdlInitError` only.
- `lib.rs` owns public exports and maps standards translation failure into adapter construction failure.
- `query.rs` owns proposition semantics and temporary consistency probes.
- `outcome.rs` owns initialization versus query-time error classification.
- `main.rs` consumes `OwlTranslationError` for export and `RustdlInitError` for reasoning initialization.

## Affected areas

- `crates/know-owl/src/{backend.rs,lib.rs,init.rs,translate.rs,query.rs,outcome.rs}`
- `crates/know-owl/tests/{relational.rs,overlap.rs}`
- `crates/know-cli/src/main.rs`
- `README.md`
- `docs/plans/feat/owl-semantic-bridge/*.md`

## Decisions

- Empty `And` is `owl:Thing`, empty `Or` is `owl:Nothing`, and singleton forms unwrap, preserving existing Boolean semantics and producing valid OWL.
- N-ary equivalent/disjoint axioms with fewer than two operands are rejected atomically; they do not have a meaningful OWL axiom form in this slice.
- Export validates Functional Syntax independently of rustdl, so standards output is not coupled to backend limitations.
- `Unsupported` and `InvalidStructure` standards translation failures reject adapter initialization as `Unsupported`; an `InvalidOutput` generated-output failure is an adapter `InternalError`.
- Negative relation, membership, subclass, equivalence, and disjoint propositions are decided by complete matching backend evaluations; temporary constraints are the complete negative proof for the three class-relation propositions.
- After adapter initialization, conversion errors belong to the adapter and are `InternalError`, never unsupported user input.

## Risks

Temporary constraints must use the same standards/backend normalization as source axioms or malformed probes could hide results. Parser validation catches invalid standards output but does not prove every OWL semantic property. Rustdl may return resource limits or errors; these must not be collapsed to `Unknown`.

## Validation

- Export and backend tests verify parse-valid source, empty/singleton Boolean and complement behavior, typed duplicate/undersized-axiom rejection, and standards-only source metadata.
- Semantic tests prove explicit overlap, anonymous existential witnesses, open cases, membership unknown, and inconsistent ontologies have the stated verdicts.
- Outcome tests prove generated probe conversion and role-chain errors are internal while direct standards-to-initialization error mapping preserves rejected input versus generated-output failures.
- Run `cargo fmt`, `cargo test -p know-owl`, affected CLI compile/tests, `cargo test --workspace --exclude cleancode`, `git diff --check`, and Clippy attempts; report exact blockers without unrelated edits.
