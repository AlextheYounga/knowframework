# Implementation plan

## Current behavior

`KnowledgeModuleSource` in `crates/know-ontology/src/source.rs` has only an ID,
schema version, declarations, and axioms. `compile::compile` in
`crates/know-ontology/src/compile.rs` collects symbols from one source and
resolves every reference against that local table. `KnowledgeModule` in
`ir.rs` is a flattened single-module IR; records retain their source
provenance, but not an explicit owner. The CLI `load_module` reads one path and
all compile-backed commands call `compile` on that source. Diagnostics and
stable codes live in `know-core`; current compiler tests cover local duplicate,
unresolved, status, and definition-cycle errors.

The corpus guides describe `<module>::<local_name>` IDs but require references
to resolve in the same file and state that imports are unavailable. The
geometry corpus is the canonical example and has no imports.

## Intended behavior

An import field on `KnowledgeModuleSource` contains ordered declarations of an
expected `ModuleId` and relative `.know` path. The CLI loader starts at the
requested root, establishes its parent directory as the package root, first
lexically checks and normalizes each import relative to its importing file, and
rejects absolute paths, non-`.know` paths, and lexical paths outside that
package root. It then canonicalizes the existing package root and each loaded
file, rejecting symlink-resolved files outside the canonical root. It parses
each reachable file once and reports missing files with the importing file and
import location.

The loader validates that each file's declared ID matches its import edge,
rejects duplicate module IDs and import cycles, and passes a parsed loaded
module graph to the ontology crate. The graph compiler validates each module
in dependency order. During resolution, a module sees its own symbols and the
symbols owned by each direct import only. A symbol owned by a loaded
transitive-only module is diagnosed as a module-boundary violation; a name that
does not belong to the local/direct symbol table receives the existing class,
relation, or entity unresolved diagnostic. Duplicate globally-qualified IDs
are rejected within each typed symbol kind (concept, relation, or entity);
identical strings in different typed namespaces remain valid. After all
modules are validated, the compiled root flattens declarations and axioms from
the entire transitive closure. This reasoning closure is deliberately broader
than source visibility, so axioms in a direct dependency retain the semantics
of that dependency's own direct imports.

The graph compile result is the root `KnowledgeModule` used by current
reasoners and exporters. It contains root plus entire-closure declarations and
axioms so cross-module reasoning works. Every IR
concept, relation, entity, and annotated axiom carries its owning `ModuleId`;
source provenance remains attached to its original record, and axiom IDs are
made stable per owning module. The source graph remains separate, so
`normalize` serializes only the requested root source and never writes imported
records into it.

## Approach

1. Extend the source schema with a small import declaration type and default an
   absent import list to empty, preserving existing RON modules.
2. Add graph input and graph-specific compile entry points in `know-ontology`.
   Keep `compile(source)` as the import-free API; when imports are present it
   returns a dedicated error explaining that a loaded graph is required.
3. Move graph topology/path-independent validation into the compiler: duplicate
   module IDs, cycles, typed-kind symbol collisions, direct visibility,
   boundary violations, and local reference resolution. Validate every module
   against its own direct imports before flattening the complete transitive
   closure for the root IR. Keep filesystem/path checks out of this crate.
4. Extend IR ownership metadata while keeping globally-qualified IDs unchanged.
   Flatten the entire validated closure only at the compiled-root boundary,
   retaining all declarations and axioms for current reasoner APIs.
5. Refactor `know-cli` loading into a filesystem graph loader shared by check,
   normalize, reason, explain, classify, export, and admission. Commands that
   compile use the graph result; normalize uses the root source serialization.
6. Settle admission on a root-plus-dependencies context: merge proposals only
   into the root source, recompile that merged root against unchanged loaded
   dependencies for structural/logical/regression validation, and persist only
   the root source on apply.
7. Add stable `know-core` diagnostic codes and path/module locations for every
   new failure class, following the existing structured diagnostic format.
8. Update compiler, loader, CLI, and reasoner-facing fixtures/tests, then
   update `.agents/README.md`, corpus format/modeling/validation guides, the
   geometry/example documentation, and README capability/boundary text.

## Responsibilities and boundaries

- `know-ontology::source`: deserialize import declarations only; no path I/O.
- `know-ontology::compile`: validate each loaded module with direct symbol
  visibility, resolve references, detect graph errors, and construct a root IR
  containing the complete validated reasoning closure with owned records.
- `know-core`: own stable diagnostic codes and existing diagnostic locations.
- `know-cli`: read files, enforce lexical and canonical package-root/path
  rules, build the parsed graph, report file-aware loader errors, and select
  root-only normalization.
- `know-reasoner`/`know-owl`: consume the existing compiled-root shape and its
  now-owned records; no filesystem or import policy belongs here.
- `know-admission`: hold the root source and unchanged loaded dependencies,
  merge proposals only into the root for recompilation and validation, and
  apply accepted changes only to the root source.
- `.agents` and README/docs: describe direct imports, visibility, safe paths,
  source-vs-compiled-root behavior, and required validation evidence.

## Affected areas

- `crates/know-ontology/src/source.rs`, `compile.rs`, `ir.rs`, `lib.rs`.
- `crates/know-core/src/lib.rs` diagnostic codes/locations.
- `crates/know-cli/src/main.rs` loader and compile call sites.
- `crates/know-admission/src/lib.rs` graph-aware structural/regression loading.
- `crates/know-ontology/tests/compile_tests.rs` plus loader/CLI and reasoning
  fixtures under existing test locations.
- `.agents/README.md`, `.agents/CORPUS_FORMAT.md`,
  `.agents/MODELING_GUIDE.md`, `.agents/VALIDATION.md`, `README.md`, and
  relevant example/package documentation.

## Decisions

- Use explicit `{ module_id, path }` import declarations rather than inferring
  ownership from paths; this makes renamed/misplaced files diagnosable and keeps
  module identity stable.
- Let only the CLI resolve paths. This keeps ontology compilation deterministic
  and reusable for in-memory callers, tests, and future non-filesystem loaders.
- Define the package root as the directory containing the requested root file;
  every nested import is first checked against that root after lexical
  normalization, then the existing root and loaded files are canonicalized and
  checked again to prevent symlink escape without introducing a package
  manifest or registry.
- Make visibility edge-based, not transitive. A direct declaration is required
  at every referencing module, preventing accidental dependency leakage.
- Flatten the entire transitive closure only in the compiled root after each
  module's direct-visibility validation, and add owner metadata rather than
  changing IDs or duplicating imported data into source files. This preserves
  dependency axiom semantics, current reasoner interfaces, and normalization
  semantics.
- Treat ID collisions as errors within each typed namespace only. Concept,
  relation, and entity IDs remain distinct because the project and OWL codec
  model them as different kinds.
- Admission owns a root source plus its already loaded dependency graph;
  proposal merge/recompile/validation changes the root view only, dependencies
  remain unchanged, and apply persists only the root source.

## Risks

- Flattening the complete closure can alter reasoner results; fixtures must
  prove direct-import entailments, dependency axioms that reference their own
  direct imports, and unchanged import-free geometry behavior.
- Existing callers may construct IR records directly; ownership fields and
  graph APIs require compatibility updates without changing canonical IDs.
- Admission operates on source modules while reasoning on a compiled graph;
  root-only merge and normalization must not accidentally persist imports.
- Lexical path checks can be bypassed by symlinks or inconsistent
  normalization; tests must cover absolute paths, `.`/`..`, nested imports,
  lexical package-root escapes, and symlink-resolved escapes.
- A cycle or duplicate ID must not cause partial graph compilation or confusing
  secondary unresolved diagnostics; topology/global validation should precede
  resolution and collect actionable related locations.

## Validation

- Compiler tests prove import-free `compile(source)`, clear rejection of
  import-bearing single-source compilation, dependency-order resolution,
  direct visibility, transitive-only rejection, complete closure flattening,
  dependency-axiom semantics, local unresolved references, duplicate IDs per
  typed kind, cycles, declared-ID mismatch, and duplicate module IDs.
- Loader/CLI tests prove missing-file diagnostics, relative `.know` enforcement,
  lexical normalization, canonical symlink-safe package-root containment,
  root-only normalize output, and graph-backed check/reason/query behavior.
- Reasoner/export tests prove imported declarations and axioms participate in
  entailment while canonical IDs and owner/provenance metadata remain intact.
- Admission tests prove root-only proposal merge, recompilation against
  unchanged dependencies, regression validation over the resulting closure,
  and root-only apply persistence.
- Corpus validation updates demonstrate the direct-import examples, intended
  cross-module query, blocked dependency leakage, and unchanged geometry
  regression commands described by `.agents/VALIDATION.md`.
- Run focused ontology, CLI, reasoner, and admission suites, then the workspace
  suite excluding `cleancode`; review the final diff for source/IR separation,
  path safety, and documentation consistency.
