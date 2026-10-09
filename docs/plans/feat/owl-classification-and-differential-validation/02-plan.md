# Plan

## Current behavior

`know-owl` exports a compiled `KnowledgeModule` to OWL Functional Syntax and
constructs `RustdlReasoner` around that serialized ontology. Its `query`
implementation probes rustdl for consistency, satisfiability, subclass,
equivalence, disjointness, membership, and relation assertions. Its
`classify` implementation is currently always `Unsupported`.

`know-reasoner` defines `ClassificationResult` and the `Reasoner` trait.
`BooleanReasoner` internalizes role-free definitions and axioms into SAT and
already computes direct named parents, children, and equivalences. Any
relational construct marks that backend unsupported.

`know-cli` has `Classify`, `ExportOwl`, and `ImportOwl` subcommands. `Classify`
currently instantiates `BooleanReasoner`; `DiffReasoner` only returns a
Phase-5 error. `know-owl` depends on `owl-dl-reasoner` and `horned-owl`, but
there is no external-oracle integration. The accepted OWL archive states that
Know's typed IR remains canonical and that incomplete rustdl results must not
be reported as `Unknown` or `Contradicted`.

## Intended behavior

The semantic bridge remains the sole boundary from `KnowledgeModule` to the
OWL backend. Rustdl classification accepts a named, vocabulary-valid concept
and evaluates every ordered pair of named concepts through the bridge. Complete
mutual entailments are quotiented into equivalence sets; deterministic
transitive reduction over the resulting strict partial order produces direct
parent and child edges. Consistency and every pairwise result needed for the
classification must be complete before `Complete` is returned. Any
`Unsupported`, `ResourceLimit`, or `InternalError` propagates without partial
vectors.

A shared differential module builds the finite corpus in canonical order. It
includes consistency, satisfiability for named concepts and finite explicit
expressions, ordered subclass/equivalence/disjoint concept pairs, entity
membership pairs, relation-holds pairs, and propositions represented directly
by module axioms. It traverses explicit expressions only to collect existing
subexpressions and never invents deeper expressions. Each backend outcome is
converted to the normalized result category, retaining a diagnostic for
non-complete outcomes.

`diff-reasoner` loads and compiles the module once, constructs both in-process
reasoners, evaluates the corpus in canonical order, prints one stable record
per query plus summary counts, and treats only explicit `Unsupported` caused
by non-overlap as a skip. `ResourceLimit` and `InternalError` are operational
failures and exit nonzero. Generated `IllTyped` queries are generator defects
and also exit nonzero. Complete category differences are semantic mismatches
and exit nonzero; skips alone do not fail the command.

External validation is a separate opt-in path enabled only by the
`diff-reasoner --external` flag. Without that flag, no `KNOW_ROBOT` discovery
occurs. With it, the command discovers the ROBOT executable from `KNOW_ROBOT`,
asks ROBOT to run HermiT over the exported ontology/probe inputs, and maps the
finite corpus to the same normalized categories. A missing or unusable
`KNOW_ROBOT` executable is a clear error and nonzero exit. Opt-in automated
tests skip clearly when the environment is absent.

For each applicable proposition, the external adapter uses the bridge's
inconsistency-first positive/negative dual consistency-probe semantics. A
positive inconsistency probe establishes entailment; a negative inconsistency
probe establishes contradiction; only completed dual consistency probes that
establish neither side produce `Unknown`. The external comparison uses
normalized categories, never explanation text.

## Approach

Add `crates/know-owl/src/differential.rs` for the reusable corpus/comparison
layer and `crates/know-owl/src/external_oracle.rs` for the opt-in process
adapter, rather than embedding query generation in the CLI. Expose
deterministic records and diagnostics to the CLI. Extend the rustdl adapter's
classification path using the existing OWL bridge and its completeness signal;
keep `ClassificationResult` construction in `know-owl` and leave the generic
trait in `know-reasoner`.

Use explicit category conversion for `ReasoningOutcome` and `Verdict`.
Treat only explicit non-overlap `Unsupported` as a skippable diagnostic;
`ResourceLimit`, `InternalError`, and generated `IllTyped` outcomes fail the
comparison, while complete verdict categories are comparable.
Sort concepts, entities, relations, expressions, query kinds, and result lists
before evaluation/output. Keep external process execution behind the opt-in
oracle adapter and pass temporary files/arguments without adding a runtime
dependency.

## Responsibilities and boundaries

* `crates/know-owl/src/lib.rs`: rustdl capability/completeness mapping,
  classification, normalized differential data types, and finite corpus
  generation/evaluation.
* `crates/know-reasoner`: retain the generic result contract; no backend
  semantics are moved into the trait.
* `crates/know-cli/src/main.rs`: parse `diff-reasoner` options, invoke the
  shared comparison service, print deterministic records, and map mismatch or
  requested-oracle failures to the process exit code.
* `crates/know-owl/tests`: role-free classification/differential coverage and
  opt-in ROBOT/HermiT validation with explicit skip behavior.
* CLI integration tests: command output and exit behavior using temporary
  fixture modules.

## Affected areas

* `crates/know-owl/src/lib.rs`
* `crates/know-owl/src/differential.rs`
* `crates/know-owl/src/external_oracle.rs`
* `crates/know-owl/tests/relational.rs` and new classification/differential
  tests
* `crates/know-cli/src/main.rs`
* CLI integration tests using temporary fixture modules
* `README.md` for the command and opt-in oracle workflow

## Decisions

1. **Use BooleanReasoner as the overlap oracle.** It is already complete for
   the represented role-free fragment and is always available in-process;
   adding another internal reference implementation would duplicate semantics.
2. **Compare normalized complete categories, not explanation text.** Backend
   explanations are not proof-equivalence contracts, while result categories
   are the stable semantic surface.
3. **Generate only finite existing vocabulary/axiom queries.** This provides
   deterministic coverage without an unbounded expression generator or a
   benchmark harness.
4. **Never fill partial classifications.** Directness requires the relevant
   hierarchy comparisons to be complete; unsupported/incomplete backends
   produce explicit outcomes instead.
5. **ROBOT/HermiT is opt-in and environment-discovered.** `KNOW_ROBOT` avoids
   bundling/downloading Java tooling and keeps external validation out of
   normal builds and runtime.
6. **Skip unsupported differential queries separately.** A role-bearing module
   must reveal the Boolean backend boundary rather than be mislabeled as a
   semantic disagreement.

## Risks

* Rustdl may return a negative answer before establishing completeness; the
  adapter must preserve its capability/resource status rather than converting
  it to `Unknown`.
* Direct-edge calculation can accidentally include transitive parents or
  equivalent nodes; tests must cover the existing geometry hierarchy and
  equivalence cases.
* Unstable iteration order would make CLI output and external diffs flaky;
  all generated and returned collections require explicit sorting.
* A module containing relations will produce many deliberate skips; summaries
  must distinguish those from mismatches and initialization failures.
* ROBOT/HermiT command, version, or output differences can make opt-in checks
  unavailable; diagnostics must include the configured executable and failure
  category without making the oracle a prerequisite.

## Validation

Validate role-free Rustdl classification against the geometry hierarchy,
including direct-vs-indirect parents, children, and equivalent classes, and
validate unsupported relational classification. Validate the corpus generator
is finite, duplicate-free, sorted, and includes explicit axiom-derived
queries. Validate Boolean/Rustdl differential output is byte-stable for the
same module, succeeds when its only non-complete outcomes are non-overlap
`Unsupported` skips, and fails on ResourceLimit, InternalError, generated
IllTyped queries, and injected complete semantic mismatches. Validate category
mapping keeps `Unknown` distinct from `Unsupported`, `ResourceLimit`, and
`InternalError`.

Run the focused crate and CLI test suites plus the workspace's normal static
checks after implementation, and run the opt-in ROBOT/HermiT tests only when
`KNOW_ROBOT` names a usable executable. The final diff must show only the
planned implementation, tests, documentation, and removal of the obsolete
untracked Acta path.
