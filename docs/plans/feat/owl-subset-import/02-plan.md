# Implementation plan

## Current behavior

`know-ontology` defines schema-version-1 source records and a typed IR in
`crates/know-ontology/src/{source,ir,compile}.rs`. The IR already represents
the requested expressions and axioms, including domain and range through
`Relation`. Compilation resolves string IDs and assigns axiom IDs from source
order.

`crates/know-owl/src/lib.rs` parses Functional Syntax for the rustdl reasoner,
exports declarations, definitions, domain/range, assertions, and class axioms,
and currently maps every ID to a `urn:know` hex IRI. Its import function always
returns an error. `crates/know-cli/src/main.rs` exposes `ImportOwl` but returns a
phase-placeholder error. Existing OWL tests cover rustdl relational reasoning;
ontology tests cover RON compilation and source round trips.
The dependent semantic bridge owns the identity-mapping direction that this
branch extends.

## Intended behavior

The importer parses one Functional Syntax ontology and validates the ontology
envelope and every axiom against the supported subset. It rejects the whole
input with a construct-specific error if any unsupported or malformed content
occurs. It expands supported names to absolute IRIs and maps each through one
shared codec:

1. Reserved canonical `urn:know` IRIs are validated strictly, decoded from
   lowercase even-length hex UTF-8, and returned as the payload in the matching
   typed ID.
2. A reserved IRI with invalid encoding or a mismatched kind is rejected.
3. Other absolute IRIs are retained verbatim as full IDs and marked external
   for export.

Non-IRI Know IDs remain exportable as canonical Know URNs. The codec treats a
non-reserved absolute ID as external, so imported external identity is emitted
verbatim rather than wrapped.

Declarations and referenced names are merged by typed identity, sorted by ID,
and given default labels equal to their ID (external IDs remain their full IRI).
Named concepts begin as `Declared`; one and only one eligible
equivalent-class definition promotes a concept to `Defined`, while the
equivalent OWL axiom is consumed into that source record's definition rather
than retained as a duplicate source axiom. Other equivalent axioms remain
axioms. `ClassAssertion(ObjectComplementOf(C) i)` converts to Know's negative
class assertion; the exporter emits that same standard Functional Syntax
shape. Relations retain domain/range expressions. All supported semantic
statements are converted exactly once, sorted by a canonical structural key,
and then compiled so generated `axiom:{index}` IDs are stable.

The module ID is always `owl-import::<sha256-lowercase-hex>`. When an ontology
IRI is present, the digest is SHA-256 of its UTF-8 bytes; otherwise it is
SHA-256 of the canonical, sorted supported-content representation. Imported
provenance retains the ontology IRI when present. The CLI reads the given file,
calls the importer, serializes the returned source with
`KnowledgeModuleSource::to_ron`, and writes only that RON to stdout.

## Approach

Extend `know-owl` with focused parsing, identity, normalization, and source
conversion helpers rather than changing the canonical ontology IR. Use the
existing horned-owl Functional Syntax parser to obtain the ontology model, but
walk its declarations and axioms through an explicit allowlist. The allowlist
handles exactly the expression and axiom forms represented by `ConceptExpr` and
`AxiomSource`; every other model variant returns an error naming the construct
and location.

Extend and reuse the semantic bridge's reversible IRI codec beside the existing
export IRI helpers; do not introduce a competing identity implementation. Both
import and export call that codec. Its boundary representation records whether
an ID is a reserved Know identity or an external absolute IRI while the Know
source schema continues to store only the lossless string ID. Export uses the
same classification to choose the original external IRI or a canonical Know
URN.

Build an intermediate normalized import collection inside `know-owl` to merge
declarations/references, identify unique definitions, sort records, compute
the canonical digest, and produce `KnowledgeModuleSource`. Keep the CLI thin:
read, delegate, serialize, and report errors. Update user documentation only
as part of implementation, not during this planning-only change.

## Responsibilities and boundaries

- `know-owl`: Functional Syntax parsing, subset allowlisting, absolute-IRI
  validation, bridge-codec reuse, normalization, defaults, sorting, digest, and
  OWL export identity selection.
- `know-ontology::source`: remains the target schema; no OWL-specific types
  enter the canonical source or typed IR.
- `know-ontology::compile`: validates generated source and supplies typed
  `KnowledgeModule` output for export/reasoning.
- `know-cli`: owns path I/O and canonical RON printing for `ImportOwl`.
- `know-owl/tests` and CLI integration coverage: prove conversion, rejection,
  identity, ordering, and round-trip behavior.

## Affected areas

- `crates/know-owl/src/lib.rs` (or focused modules extracted from it): importer,
  bridge-codec integration, canonical normalization, and external-IRI-aware
  export.
- `crates/know-owl/Cargo.toml` and workspace lockfile if the chosen SHA-256
  implementation is not already available.
- `crates/know-owl/tests/`: supported subset, malformed/cross-kind identity,
  unsupported construct, deterministic ordering, and OWL↔Know round-trip tests.
- `crates/know-cli/src/main.rs`: implement `ImportOwl` file read and RON output.
- `README.md` and relevant capability documentation: remove the import
  placeholder and document the supported subset and failure behavior.

## Decisions

- **Subset boundary:** accept only the source/IR constructs listed in
  `01-idea.md`; this matches current Know representation and prevents semantic
  weakening under the accepted OWL 2 DL direction.
- **Identity:** extend and reuse the semantic bridge's centralized codec;
  canonical Know URNs decode to typed payload IDs, while non-reserved absolute
  IRIs remain full IDs. Import and export therefore share one reversible identity
  rule and detect typed-kind misuse without competing logic.
- **Malformed reserved URNs:** any `urn:know:` IRI is reserved and must have
  exactly a supported kind and canonical lowercase even-length hex; it is never
  silently treated as external.
- **IRI resolution:** names must already be absolute after parser prefix
  expansion. No base is invented, and prefix labels are never stored as IDs.
- **Definition inference:** only a unique two-member named-plus-expression
  equivalent axiom supplies a definition and is consumed; all other named
  classes remain `Declared`, and all other equivalent axioms remain axioms.
- **Negative class membership:** recognize only
  `ClassAssertion(ObjectComplementOf(C) i)` and export Know's negative class
  assertion in that standard form.
- **Determinism:** vocabulary is ID-sorted, axioms are sorted by canonical
  structural rendering, and the module identity is always
  `owl-import::<sha256-lowercase-hex>`, hashing ontology-IRI bytes when present
  and canonical supported content otherwise.
- **Import fidelity:** every supported semantic statement is retained exactly
  once; a selected equivalent-class definition is represented by the concept
  definition instead of a duplicate equivalent axiom. Unsupported content
  aborts the import.

## Risks

- Horned-owl model variants may expose syntax that looks structural but cannot
  be represented in Know; exhaustive allowlist tests must ensure it errors.
- Treating absolute IDs as external on export creates an intentional boundary
  for native Know IDs that themselves look like absolute IRIs; the codec rule
  must be documented and tested consistently.
- Equivalent-class candidate detection can accidentally create invalid
  `Defined` records or duplicate/lose semantics; consume only the unique
  named-plus-expression statement and test ambiguous and named-to-named cases.
- Input declaration order and parser set iteration can make module/axiom IDs
  unstable unless normalization sorts before digesting and source generation.
- OWL escaping and absolute-IRI validation can allow malformed output or
  identity changes; round-trip tests must include punctuation and Unicode IDs.

## Validation

- Unit/integration tests import every supported expression and axiom form,
  including complement-shaped negative class membership, and assert every
  supported semantic statement appears exactly once in generated source.
- Codec integration tests prove bridge-codec reuse, canonical Know URN
  decode/encode losslessness, reject odd,
  uppercase, empty, non-hex, malformed reserved URNs and cross-kind use, and
  preserve arbitrary external absolute IRIs exactly.
- Rejection tests cover annotations, imports, version metadata, data/literal
  constructs, unsupported property axioms, anonymous individuals, and relative
  IRIs; each reports an explicit error and produces no partial module.
- Determinism tests permute input declaration/axiom order and compare module ID,
  source vectors, canonical RON, and exported OWL; they also verify the module
  hash uses ontology IRI bytes when present and canonical content otherwise.
- Round-trip tests perform supported OWL → source → compiled IR → OWL and verify
  canonical Know identity, external IRI preservation, standard negative class
  assertion syntax, and single emission of consumed definitions.
- CLI coverage verifies `import-owl` prints parseable canonical RON on success
  and exits nonzero with an explicit error on unsupported input.
- Run focused OWL/ontology/CLI tests, workspace regression tests, formatter,
  clippy, and review the final diff after implementation.
