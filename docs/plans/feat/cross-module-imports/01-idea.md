# Explicit cross-module imports

## Request

Add explicit Know ontology-module imports. A `.know` module import names both
the expected module ID and a relative `.know` path. The CLI loads the root and
all transitive imports; the ontology compiler receives the resulting loaded
graph rather than accessing the filesystem. Direct imports expose their public
symbols for source resolution, while transitive imports remain unavailable to a
module's source references until directly declared. The compiled root
nevertheless includes the entire loaded transitive closure for reasoning.

## Problem

The source schema and compiler currently support only one module at a time.
Every reference must resolve in that file, the CLI loads only one path, and
the corpus rules explicitly prohibit cross-module ontology references. This
prevents separate ontology packages from sharing stable globally-qualified
concepts, relations, entities, and axioms while giving no precise diagnostics
for dependency or module-boundary mistakes.

## Definitions

**Import declaration:** A source-level declaration containing an expected
module ID and a relative path ending in `.know`. It declares a direct
dependency; it is not an OWL import, registry lookup, alias, or re-export.

**Loaded module graph:** The filesystem-independent compiler input containing
the parsed root and every transitively imported source module, keyed by module
ID and linked by validated import declarations.

**Direct visibility:** A module may resolve its own symbols and symbols owned by
modules it directly imports. A loaded transitive dependency is not visible
without a direct declaration in the referencing module.

**Reasoning closure:** After every module is validated using its own direct
visibility, the compiled root flattens declarations and axioms from the entire
loaded transitive dependency closure. This closure is an IR/reasoning input,
not a change to source visibility or source normalization.

**Compiled root:** The IR returned for the requested root module. It contains
the declarations and axioms from the entire reasoning closure, while each
record retains its owning module and source provenance. Imported records are
compilation input, not records written into the root source during
normalization.

## Desired outcome

CLI checks, normalization, reasoning, classification, export, and admission
work from a root file with valid explicit imports. Compilation is dependency
ordered and rejects missing files, declared-ID mismatches, duplicate module IDs,
cycles, duplicate symbols within a typed namespace, transitive-only references,
and unresolved local references with module-aware diagnostics. `compile(source)`
continues to compile import-free modules and clearly rejects import-bearing
sources because no loader/graph was supplied.

## Scope

- Add source import declarations and graph-aware ontology compilation.
- Add filesystem-only CLI loading with package-root and path safety checks.
- Validate each module with direct-only source visibility, then flatten the
  entire transitive closure for reasoning without dependency leakage in source
  resolution.
- Preserve canonical global IDs, ownership, provenance, and imported axioms in
  the compiled root IR.
- Add diagnostics and focused compiler/loader/CLI coverage.
- Update corpus contribution rules and examples to describe the new boundary.

## Constraints

- Import paths are relative, lexically normalized, must end in `.know`, and
  cannot escape the package root. The loader canonicalizes the existing
  package root and every loaded file after lexical checks, rejecting
  symlink-resolved paths outside the canonical root. There is no network,
  package registry, or version solver.
- Path resolution is restricted to the filesystem/CLI loader. The ontology
  compiler remains filesystem-independent and receives parsed graph data.
- Compile imported modules in dependency order before resolving the root.
- Direct imports expose public symbols; there are no aliases, re-exports, or
  visibility controls.
- Imported declarations/axioms must be available to reasoners without changing
  canonical globally-qualified IDs or root source serialization.
- Global ID collisions are rejected independently within concepts, relations,
  and entities; the same string may occur once in each typed namespace.
- Admission holds the root source and loaded dependency graph together. A
  proposal merges only into the root; validation recompiles that merged root
  against unchanged dependencies, and apply persists only the root source.
- Existing import-free single-module behavior remains compatible.

## Exclusions

OWL ontology imports, remote imports, module versions, package registries,
version solving, aliases, re-exports, mutable dependency resolution, and
general visibility controls are not part of this change.
