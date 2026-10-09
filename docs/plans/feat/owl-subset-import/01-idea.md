# OWL supported-subset import

## Request

Implement supported-subset OWL 2 Functional Syntax import, stable identity
mapping, and import/export round trips on top of the Know IR and existing OWL
semantic bridge. Add `know import-owl` output as canonical RON. Unsupported OWL
constructs must fail explicitly and never be discarded.

## Problem

Know can export its current OWL-aligned fragment, but `know-owl` rejects every
import and the CLI command is only a placeholder. The current exporter wraps
every Know ID in a Know URN, which cannot preserve arbitrary external OWL IRIs
losslessly. Import cannot yet be a reliable interchange boundary.

## Definitions

**Supported subset:** OWL Functional Syntax containing named classes, named
object properties, named individuals, Boolean class expressions (`and`, `or`,
`not`), existential and universal object restrictions, subclass,
equivalent-class and disjoint-class axioms, positive and negative class and
object-property assertions, and object-property domain/range axioms.

Negative class membership is represented only by the Functional Syntax shape
`ClassAssertion(ObjectComplementOf(C) i)` and maps to Know's
`NegativeClassAssertion`; there is no separate OWL `NegativeClassAssertion`
axiom in this subset. Negative object-property membership uses OWL's standard
`NegativeObjectPropertyAssertion`.

**Canonical Know IRI:**
`urn:know:{concept|relation|entity}:{lowercase-even-length-hex-UTF8-Know-ID}`.
Its decoded payload is the complete corresponding typed Know ID. The `kind`
component is a type boundary, not an ID namespace hint.

**External IRI identity:** Any supported absolute IRI outside the reserved
`urn:know:` namespace. Its complete IRI string is stored as the corresponding
Know ID and emitted unchanged on export.

**Unsupported construct:** Any OWL syntax or semantic construct outside the
supported subset, including annotations, imports, anonymous individuals,
unresolved relative IRIs, data properties/literals, property axioms beyond
domain/range, and ontology version metadata. It is an import error, never an
omitted record.

**Unique equivalent-class definition:** An `EquivalentClasses` axiom with
exactly one named class and exactly one non-named class expression, when that
named class has exactly one such candidate across the ontology. It supplies the
class record's definition and `Defined` status, and is consumed rather than
retained as a second source statement. All other named classes are `Declared`.

## Desired outcome

Supported OWL ontologies import into valid Know source modules whose canonical
RON preserves every supported semantic statement exactly once. Re-exporting
the compiled module reproduces canonical Know IRIs and external IRIs without
identity loss; canonical supported OWL input has a deterministic
import/export representation.
Malformed canonical Know URNs, typed-kind misuse, and unsupported input produce
clear failures. `import-owl` prints only canonical RON on success.

## Scope

- Parse and validate the supported OWL Functional Syntax subset.
- Convert supported structural content into schema-version-1 Know source data.
- Extend and reuse the semantic bridge's centralized reversible IRI identity
  codec from import and export; do not create competing identity logic.
- Derive deterministic module IDs, labels, provenance, ordering, and axiom IDs.
- Wire the existing CLI command to file input and canonical RON output.
- Add focused import, identity, rejection, and round-trip coverage.

## Constraints

- Know's typed IR remains canonical; rustdl/horned OWL model types remain
  boundary implementation details.
- The accepted OWL direction in
  `docs/agents/archive/0003-owl-2-dl-semantic-target.md` requires unsupported
  features to be reported rather than approximated.
- Canonical URN payloads are UTF-8 bytes encoded as lowercase, even-length hex
  and must decode losslessly to the typed ID.
- External absolute IRIs are stored verbatim as full typed IDs; no labels,
  slugs, case folding, or generated namespace may replace them.
- Import must reject anonymous individuals, relative IRIs without a resolved
  base, ontology annotations/import closure, and prefix tokens used as identity.
- Source defaults are deterministic: module ID is always
  `owl-import::<sha256-lowercase-hex>`, hashing the ontology IRI UTF-8 bytes
  when present and otherwise the canonical supported-content representation;
  labels default to the decoded ID or original IRI; imported provenance retains
  the ontology IRI when present; axiom order is explicit and stable.
- This work is planning only on the feature branch and must commit only the
  three planning documents.

## Exclusions

Full OWL 2 DL import/export, data properties and datatypes, property
characteristics/hierarchies/chains/inverses, cardinalities, nominals,
same/different individuals, annotations, ontology imports, prefix-based
identity, anonymous individuals, base-IRI resolution, lexical bindings,
provenance beyond the available imported source field, and changes to the
reasoning contract are excluded.
