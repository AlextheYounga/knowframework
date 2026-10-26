# Tasks

## Implementation

- [x] Add strict typed IRI translation and atomic source coverage.
- [x] Publish standards-only `translate_owl`/`OwlTranslation` metadata and keep rustdl NNF adaptation private and downstream of successful translation.
- [x] Expose `OwlTranslationError` from standards export, parser-validate its output, and retain `RustdlInitError` only for adapter construction.
- [x] Normalize empty/singleton Boolean expressions and reject undersized equivalent/disjoint axioms before rendering.
- [x] Replace declared-entity negative subclass/equivalence checks with temporary-constraint consistency proofs; add matching disjointness proof and complete error propagation.
- [x] Classify post-initialization probe conversion and role-chain failures as `InternalError`.
- [x] Update CLI error ownership and README capability description.

## Validation

- [x] Add export cardinality/parser-validity cases and backend execution coverage for empty, singleton, and complemented Boolean expressions.
- [x] Add explicit and anonymous overlap contradiction cases plus open subclass/equivalence/disjoint and unknown membership cases.
- [x] Add inconsistent ontology and generated-probe status boundary cases.
- [x] Run formatting, focused package tests, affected CLI checks, workspace tests, diff check, and Clippy attempts with recorded results.

## Completion

- [x] Review the final diff for public API separation, complete OWL proof semantics, private adapter artifacts, and exclusions.

## Completion notes

The earlier implementation established the slice structure but its declared-individual negative proof, exporter lifecycle error, n-ary rendering policy, and coupled standards/backend translation were superseded by clarification 04 and its authoritative updates. `cargo test -p know-owl` passed (8 unit, 4 representative overlap, 9 relational), including direct lifecycle mapping and rustdl Boolean identity execution. `cargo fmt -p know-owl -- --check`, `cargo check -p know-cli`, `cargo test -p know-cli`, `cargo test --workspace --exclude cleancode`, and `git diff --check` passed. `cargo fmt --all -- --check` remains blocked by pre-existing formatting in `know-ontology` and `know-reasoner`; those files were not changed. `cargo clippy -p know-owl --all-targets --all-features --no-deps` now passes with no warnings. Workspace-wide Clippy remains subject to pre-existing blockers outside `know-owl`, including denied lints in `tests/cleancode` and `know-ontology`.
