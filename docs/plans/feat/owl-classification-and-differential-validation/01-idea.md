# Idea

## Request

After the semantic bridge/import work, map Rustdl hierarchy classification into
Know's `ClassificationResult`. Add a deterministic `diff-reasoner` CLI over a
finite query corpus, compare `BooleanReasoner` with `RustdlReasoner` for every
applicable generated query, and add optional differential validation against a
mature external OWL reasoner. Use ROBOT/HermiT as the opt-in external oracle,
discovered through `KNOW_ROBOT`.

## Problem

Rustdl currently answers individual OWL-aligned propositions but exposes no
Know classification result, while the CLI classification command is wired only
to the Boolean backend and `diff-reasoner` is a stub. There is consequently no
deterministic check that the two in-process backends agree on their shared
role-free semantics, and no explicit operator-facing distinction between an
unsupported query, an incomplete/resource-limited result, and a semantic
mismatch.

## Definitions

### Finite query corpus

A deterministic, duplicate-free set derived only from the module's named
concepts, entities, relations, and explicit module axioms (including their
finite subexpressions). It does not synthesize expressions to arbitrary depth.

### Applicable query

A corpus query whose ontology and expression use only constructs supported by
both selected in-process reasoners. Relational queries are retained in the
corpus but reported as skipped when the Boolean backend cannot evaluate them.

### Normalized result category

The stable comparison value for one backend/query pair: `Entailed`,
`Contradicted`, `Unknown`, `Inconsistent`, `IllTyped`, `Unsupported`,
`ResourceLimit`, or `InternalError`. `Unknown` is a complete logical result;
backend inability is never normalized to `Unknown`.

### Complete classification

A `ClassificationResult` containing direct named superclasses, direct named
subclasses, and equivalent named classes only when the backend has established
all relations needed for the requested result. Incomplete or unsupported
classification is returned as `ResourceLimit` or `Unsupported`, never as a
partial `Complete` result.

### Differential disposition

Only explicit `Unsupported` caused by a query being outside the two-backend
overlap is skipped. `ResourceLimit` and `InternalError` are operational
failures; generated `IllTyped` queries are generator defects; and complete
category differences are semantic mismatches. Each causes `diff-reasoner` to
exit nonzero.

### External oracle

An optional test/validation process using the ROBOT executable configured by
`KNOW_ROBOT`, with HermiT selected by ROBOT. It is not a Cargo dependency,
build prerequisite, or production runtime service.

## Desired outcome

`RustdlReasoner::classify` returns a deterministic, sorted
`ClassificationResult` for supported named concepts and explicit diagnostics
otherwise. `know diff-reasoner <module>` emits stable corpus/query outcomes,
counts complete comparisons and skipped queries separately, and exits
nonzero on any Boolean/Rustdl semantic mismatch. An explicitly requested
external oracle is enabled only with `--external`; without that flag no
`KNOW_ROBOT` discovery occurs. With the flag, it validates the finite corpus
when `KNOW_ROBOT` is usable and fails clearly when it is absent/unusable;
opt-in tests skip with an explicit diagnostic when that oracle is unavailable.

## Scope

* Rustdl-to-Know hierarchy classification mapping after the semantic
  bridge/import boundary.
* Explicit completeness, unsupported, resource-limit, and backend-error
  diagnostics for classification and query comparison.
* A shared deterministic finite corpus generator and normalized result model.
* Boolean-vs-Rustdl differential CLI behavior and exit policy.
* Opt-in ROBOT/HermiT differential validation and its skipped/unavailable
  reporting.
* Focused reasoner, corpus, CLI, and opt-in oracle tests.

## Constraints

* `BooleanReasoner` is the always-available in-process overlap oracle for
  represented role-free Boolean cases.
* Query generation uses named concepts/entities/relations and module axioms;
  it does not perform unbounded expression synthesis.
* Output ordering and diagnostic wording/categories are deterministic.
* External validation is discovered only through `KNOW_ROBOT` and is never
  required for Cargo builds, normal CLI use, or production runtime behavior.
* Direct hierarchy and equivalence claims are emitted only with backend
  completeness support.

## Exclusions

Full OWL 2 DL implementation, import implementation itself, bundling or
downloading Java tools, production dependence on ROBOT/HermiT, proof
equivalence, benchmarking, and performance work are excluded. The change does
not make a partial backend appear complete and does not replace the canonical
Know IR with rustdl types.
