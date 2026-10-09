# Tasks

## Implementation

- [ ] Add `ModuleImport` source schema with expected module ID and relative path; preserve empty-import RON compatibility.
- [ ] Add graph source types and filesystem-independent compiler entry points; retain single-module compile behavior and its explicit imports-require-graph error.
- [ ] Implement graph topology validation and dependency ordering for duplicate module IDs, cycles, and declared-ID mismatches.
- [ ] Extend symbol collection/resolution for owned IR records, typed-kind-only collisions, direct-import visibility, transitive-only boundary diagnostics, and local unresolved references.
- [ ] Validate every module under its own direct-import visibility, then flatten the entire validated transitive closure of declarations and axioms into the compiled root for reasoning, preserving dependency axiom semantics and ownership metadata.
- [ ] Add stable core diagnostic codes and module/file-aware locations for all import and boundary failures.
- [ ] Refactor CLI loading to enforce relative `.know` paths, lexical normalization, canonical package-root containment including symlink resolution, missing-file reporting, and shared transitive graph loading.
- [ ] Update CLI compile-backed commands and admission to use the loaded graph; hold root plus unchanged dependencies, merge/recompile/validate proposals against that context, and keep normalize/apply root-source-only.
- [ ] Update fixtures and tests for valid transitive graphs, direct-import requirements, complete closure reasoning, dependency axioms, typed namespace collisions, every diagnostic class, provenance/ownership, and normalization isolation.
- [ ] Update `.agents/README.md` and the corpus format, modeling, and validation guides with explicit import contribution rules and examples; update README capability/boundary documentation.

## Validation

- [ ] Run focused ontology compiler tests and verify all import graph, direct-visibility, complete-closure, typed-collision, cycle, and compatibility assertions pass.
- [ ] Run CLI/loader tests and verify lexical plus canonical symlink-safe path handling, root-only normalization, and graph-backed commands produce expected diagnostics/results.
- [ ] Run reasoner/export and admission tests and verify complete-closure axioms are usable, dependencies remain unchanged during proposal validation, and apply writes only root source.
- [ ] Validate the documented corpus example and geometry regression checks, including a deliberately blocked transitive dependency reference.
- [ ] Run the workspace suite excluding `cleancode` and review diagnostics for stable codes, useful locations, and no misleading secondary errors.

## Completion

- [ ] Review the final diff to confirm only requested implementation, tests, corpus-rule documentation, README/docs, and plan files changed.
- [ ] Confirm old untracked Acta planning templates are removed and no generated imported source is committed.
- [ ] Record implementation deviations, validation evidence, and deliberately unfinished exclusions in completion notes.

## Completion notes

Not applicable during planning; implementation and validation are intentionally pending human approval of this record.
