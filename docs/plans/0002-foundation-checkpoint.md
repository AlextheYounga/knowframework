# Foundation Checkpoint

## Status

The repository has moved from an empty Rust binary to a working Phase 1 and
Phase 2 foundation for Know. The source schema compiles into a typed
ontology IR, the Boolean concept fragment has a real SAT-backed reasoner, the
geometry knowledge package is executable, and the admission pipeline and CLI
can exercise the implemented pieces.

This is not yet the complete Know system. Relational ALC reasoning, OWL
interoperability, and several admission policies remain deliberately open.

## Architectural Decisions

The work retained the central decisions from the architecture plan:

- Know is a Rust workspace with narrowly scoped crates.
- `.know` files are RON-encoded Rust source structs, not a custom language.
- Source data uses strings; compilation resolves names into typed IDs.
- Canonical concepts are distinct from ambiguous natural-language word forms.
- LLM output is untrusted and must pass compiler, reasoner, and admission
  checks before it can be accepted.
- Reasoning uses the open-world assumption: failure to prove a statement is
  `Unknown`, not `Contradicted`.
- Unsupported reasoning must be reported as `Unsupported`, never disguised as
  `Unknown`.

The original idea describes a much larger self-editing proof language. The
current implementation intentionally starts with a documented OWL-aligned
Boolean fragment rather than pretending that the full proof kernel or natural
language layer already exists.

## Implemented Foundation

### Workspace and crates

The root Cargo manifest is now a workspace containing:

- `know-core`
- `know-ontology`
- `know-lexicon`
- `know-reasoner`
- `know-admission`
- `know-owl`
- `know-cli`
- `tests/fixtures` (`fixtures` crate)

The old placeholder `src/main.rs` binary was removed. Shared dependencies are
centralized in the workspace manifest, with Serde/RON, `thiserror`, and Clap
providing the basic infrastructure.

### Core model

`know-core` defines separate opaque ID types for concepts, entities,
relations, modules, axioms, and languages. It also provides provenance,
source locations, diagnostics, stable diagnostic codes, and display formatting
for diagnostics.

### Ontology source and compiler

`know-ontology` now has two layers:

- `source`: RON-deserializable records whose references are strings.
- `ir`: validated records and expressions whose references use typed IDs.

The compiler currently performs:

- schema-version validation,
- duplicate ID detection,
- status/definition consistency checks,
- concept, relation, and entity name resolution,
- unresolved-reference diagnostics,
- definition-cycle detection,
- collection of multiple diagnostics before failing.

RON round-trip coverage exists for source modules. Cross-module imports are
not part of the current schema; references must resolve within one module.

### Boolean reasoner

`know-reasoner` contains a real Stage 2 implementation rather than only
interfaces. `BooleanReasoner` translates the role-free concept fragment into
propositional formulas and uses a small DPLL-style SAT solver to answer:

- subclass entailment,
- equivalence,
- disjointness,
- satisfiability,
- class membership,
- consistency,
- open-world unknowns.

It returns explanations with supporting axiom IDs, inference steps, and model
notes where applicable. It also provides direct classification of named
concepts. `EXISTS`, `FOR_ALL`, and relation propositions are explicitly
reported as unsupported until a complete Stage 3 ALC procedure exists.

The verdict and outcome types distinguish `Entailed`, `Contradicted`,
`Unknown`, `Ambiguous`, `IllTyped`, and `Inconsistent` from execution states
such as `Unsupported`, `ResourceLimit`, and `InternalError`.

### Lexical layer

`know-lexicon` defines lexical forms, one-to-many concept bindings, context
hints, resolution evidence, and a `Resolver` trait. `ContextResolver` is the
first concrete strategy:

- no-context polysemy remains ambiguous,
- a uniquely supported context resolves a candidate,
- an explicit domain hint has greater weight than one incidental hint,
- language mismatches and unknown words are handled explicitly,
- lexical ambiguity never becomes a logical union.

### Admission pipeline

`know-admission` now merges a proposal with a base module and records the five
planned stages:

1. Structural compilation and reference validation.
2. Lexical binding validation.
3. Ontological validation placeholder. Grounding compatibility rules are not
   yet specified, so this stage currently passes vacuously.
4. Logical consistency and satisfiability checks, including warnings for a
   proposed concept that collapses into an existing concept.
5. Regression checks against previously expected verdicts.

The pipeline produces an auditable `AdmissionRecord` with stage results,
decision, changed verdicts, generator identity, timestamp, and provenance.
It supports acceptance, acceptance with warnings, rejection, conflict, and
deferred decision variants. Positive criteria such as stability across
independent generator runs are still unspecified.

### CLI and knowledge package

`know-cli` now exposes working commands for:

- `check`,
- `normalize`,
- `reason`,
- `classify`,
- `explain`,
- `resolve`,
- `admit`.

`export-owl` is present but still depends on a stub. `import-owl` and
`diff-reasoner` return explicit not-implemented errors.

The geometry package includes `Know.toml`, a RON `concepts.know` module, a
lexicon, and a Pentagon proposal. The programmatic fixture covers the
intended foundational cases: square inheritance, polygon/circle
disjointness, unsatisfiable intersections, inconsistent entities, open-world
unknowns, explanations, and classification.

## Tests and Verification

The repository includes:

- ontology compiler unit/integration tests,
- Boolean geometry acceptance tests,
- lexical polysemy and context-resolution tests,
- admission pipeline tests,
- workspace clean-code/lint tests.

The session verified a clean `cargo build`, successful geometry parsing with
`know check`, and successful CLI `reason` and `classify` smoke runs.

A later workspace test run showed the implementation tests passing but the
clean-code `no_legacy_terms` check failing because it flags the intentional
`Deprecated` enum/status and related documentation. This is a test-policy
mismatch to resolve, not evidence that the semantic tests are failing.

## Deliberately Unfinished

- Complete relational ALC reasoning for `EXISTS`, `FOR_ALL`, and relation
  assertions.
- Grounding-category compatibility semantics and type checking.
- OWL Functional Syntax export/import and stable Know-ID-to-IRI mapping.
- Differential testing against a mature OWL reasoner.
- A formal query language beyond the current small CLI query grammar.
- Cross-module loading and imports.
- Immutable, content-addressed knowledge versions and revision history.
- Fully specified admission stability, conflict arbitration, and audit-log
  persistence.
- Natural-language parsing, embeddings, probabilistic truth, custom grammar,
  and the larger proof-kernel/self-editing language described in `docs/idea.md`.

These areas should remain explicit TODOs. They should not be filled with
approximate reasoning that would weaken the soundness claims of the project.

## Recommended Next Steps

1. Resolve the clean-code test’s treatment of the intentional `Deprecated`
   status, then keep `cargo test --workspace` green.
2. Strengthen the ontology/compiler contract around versioning and module
   boundaries before adding imports.
3. Specify the exact Stage 3 ALC fragment and implement or integrate a
   complete tableau-style procedure rather than extending graph traversal.
4. Add the relational semantic tests before exposing relational features in
   admission or the CLI.
5. Design OWL ID mapping and differential tests only after the supported
   relational fragment is fixed.
6. Specify admission stability and conflict policy before making the pipeline
   responsible for persistent cumulative knowledge.
