# Know Framework Groundwork Plan

## Objective

Build **Know** as a native Rust semantic-coherence engine that maintains a discrete, cumulative body of formal word-sense definitions and abstract knowledge.

Know should allow an LLM to:

1. Propose a word sense, definition, axiom, or formal statement.
2. Express that proposal as structured data.
3. Validate it against the existing knowledge body.
4. Admit it automatically when it satisfies the configured coherence rules.
5. Record enough information for later human auditing.
6. Check proposed statements before the LLM presents them as valid.

Know must eventually return:

```rust
pub enum Verdict {
    Entailed,
    Contradicted,
    Unknown,
    Ambiguous,
    IllTyped,
    Inconsistent,
}
```

This phase should establish the semantic kernel and knowledge representation. It should not attempt to model all of English, perform unrestricted natural-language parsing, or invent a custom programming language.

---

# 1. Core architectural decision

Implement Know entirely in Rust.

Use `.know` files as **RON-encoded serialized Rust data**, not as a custom language.

```text
.know file
    encoding: RON
    schema: Rust structs and enums
    semantics: Rust validator and reasoner
```

The processing pipeline should be:

```text
LLM structured output or human-edited .know file
                ↓
            RON parser
                ↓
        Serde deserialization
                ↓
          Canonical Rust IR
                ↓
      Structural and name validation
                ↓
        Semantic coherence checks
                ↓
             Reasoner
                ↓
      Verdict, explanation, audit record
```

The `.know` extension is a Know project convention. The contents are parsed with the Rust `ron` crate.

Do not build:

* A custom lexer
* A custom parser
* A custom grammar
* A formatter for a new language
* A macro system
* Functions, loops, or conditionals inside knowledge files
* User-defined operators
* A `.know` compiler separate from the Rust engine

The Rust data structures are the authoritative schema. Changing the schema requires changing Rust code and tests.

---

# 2. Design principles

## 2.1 Rust contains the machinery

Rust owns:

* Knowledge schema
* Parsing through RON and Serde
* Name resolution
* Validation
* Normalization
* Logical inference
* Consistency checking
* Admission policies
* LLM orchestration
* Provenance
* Audit history
* Embedding generation
* CLI tooling
* Tests
* Schema migrations

## 2.2 `.know` contains the knowledge

`.know` files contain only serialized declarations such as:

* Concepts
* Complete definitions
* Partial classifications
* Axioms
* Relations
* Lexemes
* Senses
* Contexts
* Claims
* Provenance references

They must not contain executable behavior.

## 2.3 Knowledge is declarative

Declaration order must not affect semantic results.

The same valid `.know` files, engine version, schema version, and configuration must always produce the same canonical knowledge body.

## 2.4 LLM-driven, automatically admitted

The LLM is the primary knowledge-extraction mechanism.

Human approval should not be required for every accepted definition. Instead:

```text
LLM proposal
    ↓
automated structural validation
    ↓
automated semantic validation
    ↓
automated adversarial checks
    ↓
admission policy
    ↓
accepted knowledge or escalation
```

Humans should be able to:

* Audit accepted knowledge
* Inspect explanations
* Correct a definition
* Add distinctions or examples
* Reject or supersede accepted material
* Tighten admission policies

Human review is an auditing and correction mechanism, not the normal ingestion path.

---

# 3. Cargo workspace

Create the following workspace:

```text
know/
├── Cargo.toml
├── crates/
│   ├── know-core/
│   ├── know-engine/
│   ├── know-admission/
│   ├── know-cli/
│   └── know-test-support/
├── knowledge/
│   └── geometry/
│       ├── Know.toml
│       ├── concepts.know
│       ├── lexicon.know
│       └── contexts.know
├── tests/
│   └── geometry/
├── docs/
│   ├── semantics.md
│   ├── architecture.md
│   ├── file-format.md
│   └── admission.md
└── examples/
```

## `know-core`

Owns the canonical Rust data model:

* Identifiers
* Symbols
* Source-level schema
* Validated IR
* Concepts
* Expressions
* Relations
* Definitions
* Axioms
* Lexemes
* Senses
* Contexts
* Claims
* Provenance
* Propositions
* Verdicts
* Diagnostics
* Explanations

This crate should contain little or no reasoning logic.

## `know-engine`

Owns:

* Module loading
* Name resolution
* Structural validation
* Type checking
* Definition dependency analysis
* Normalization
* Inference
* Satisfiability checks
* Consistency checks
* Query evaluation
* Explanation generation

## `know-admission`

Owns the automated LLM admission pipeline:

* Candidate creation
* Proposal isolation
* Repeated extraction
* Counterexample generation
* Stability checks
* Regression analysis
* Admission policies
* Acceptance, rejection, or escalation
* Audit record generation

The first version may use mocked LLM responses, but the architecture should reflect the eventual automated workflow.

## `know-cli`

Provide commands such as:

```text
know check knowledge/geometry
know validate knowledge/geometry
know query knowledge/geometry query.json
know explain knowledge/geometry query.json
know canonicalize knowledge/geometry
know audit show <admission-id>
know audit concept geometry::Square
```

## `know-test-support`

Owns:

* Geometry fixtures
* Typed builder helpers
* Semantic assertion helpers
* Property-based generators
* Finite-model reference implementation
* Mutation-test utilities
* Snapshot helpers

---

# 4. Separate source schema from validated IR

Do not deserialize directly into the optimized reasoner representation.

Use two layers.

## 4.1 Source schema

The source schema should preserve names, metadata, source locations, and human-readable structure.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeModuleSource {
    pub schema_version: u32,
    pub module: QualifiedName,
    pub concepts: Vec<ConceptSource>,
    pub relations: Vec<RelationSource>,
    pub axioms: Vec<AxiomSource>,
    pub lexemes: Vec<LexemeSource>,
    pub contexts: Vec<ContextSource>,
    pub claims: Vec<ClaimSource>,
}
```

Names may remain strings or validated symbol wrappers at this layer.

## 4.2 Validated IR

After validation, compile the source schema into a resolved IR:

```rust
pub struct KnowledgeModule {
    pub id: ModuleId,
    pub concepts: Vec<Concept>,
    pub relations: Vec<Relation>,
    pub axioms: Vec<Axiom>,
    pub lexemes: Vec<Lexeme>,
    pub contexts: Vec<Context>,
    pub claims: Vec<Claim>,
}
```

Use opaque numeric or interned identifiers internally:

```rust
pub struct ConceptId(u32);
pub struct RelationId(u32);
pub struct EntityId(u32);
pub struct LexemeId(u32);
pub struct SenseId(u32);
pub struct ContextId(u32);
pub struct ModuleId(u32);
```

This separation prevents malformed or unresolved source data from entering the reasoner.

---

# 5. Canonical knowledge schema

## 5.1 Concepts

Represent primitive and defined concepts explicitly:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConceptSource {
    Primitive {
        name: String,
        annotations: Annotations,
    },

    Declared {
        name: String,
        parents: Vec<ConceptExprSource>,
        annotations: Annotations,
    },

    Defined {
        name: String,
        definition: ConceptExprSource,
        annotations: Annotations,
    },
}
```

The distinction matters:

```text
Primitive:
    accepted as foundational within this knowledge version

Declared:
    has necessary classifications but no complete definition

Defined:
    has necessary and sufficient conditions
```

## 5.2 Concept expressions

Start with a deliberately restricted algebra:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConceptExprSource {
    Named(String),
    And(Vec<ConceptExprSource>),
    Or(Vec<ConceptExprSource>),
    Not(Box<ConceptExprSource>),
}
```

Do not add quantifiers, cardinality, arithmetic, temporal logic, fuzzy membership, recursive rules, or higher-order concepts during the groundwork phase.

## 5.3 Axioms

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomSource {
    pub name: String,
    pub statement: StatementSource,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatementSource {
    Subclass {
        child: ConceptExprSource,
        parent: ConceptExprSource,
    },

    Equivalent {
        left: ConceptExprSource,
        right: ConceptExprSource,
    },

    Disjoint {
        left: ConceptExprSource,
        right: ConceptExprSource,
    },

    InstanceOf {
        entity: String,
        concept: ConceptExprSource,
    },

    NotInstanceOf {
        entity: String,
        concept: ConceptExprSource,
    },
}
```

Definitions and axioms must remain distinct even when they compile into related internal constraints.

## 5.4 Relations

Start with typed binary relations:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationSource {
    pub name: String,
    pub subject: ConceptExprSource,
    pub object: ConceptExprSource,
    pub annotations: Annotations,
}
```

V1 needs relation declarations mainly for proposition type checking.

Do not initially add:

* Transitive relations
* Symmetric relations
* Inverse relations
* Cardinality constraints
* Recursive relation rules

## 5.5 Lexemes and senses

Words must never map directly to concepts.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexemeSource {
    pub form: String,
    pub language: String,
    pub senses: Vec<SenseSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenseSource {
    pub id: String,
    pub concept: String,
    pub context: Option<String>,
    pub gloss: Option<String>,
    pub examples: Vec<String>,
    pub contrasts: Vec<String>,
    pub provenance: Provenance,
}
```

The required chain is:

```text
word form
    → lexical sense
    → formal concept
```

## 5.6 Contexts

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    pub name: String,
    pub included_modules: Vec<String>,
    pub preferred_senses: Vec<SensePreferenceSource>,
}
```

Contexts may prefer or permit senses. They must not silently change formal concept definitions.

## 5.7 Claims

Empirical claims must remain separate from definitions and abstract axioms:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimSource {
    pub name: String,
    pub statement: StatementSource,
    pub provenance: Provenance,
    pub validity: Option<ValidityPeriod>,
    pub jurisdiction: Option<String>,
    pub status: ClaimStatus,
}
```

A claim may participate in reasoning only when the active evidence policy includes it.

---

# 6. Example `.know` file

The file may be named:

```text
knowledge/geometry/concepts.know
```

Its contents are RON:

```rust
KnowledgeModuleSource(
    schema_version: 1,
    module: "geometry",

    concepts: [
        Primitive(
            name: "SpatialObject",
            annotations: (),
        ),

        Declared(
            name: "Figure",
            parents: [
                Named("SpatialObject"),
            ],
            annotations: (),
        ),

        Declared(
            name: "Polygon",
            parents: [
                Named("Figure"),
            ],
            annotations: (),
        ),

        Declared(
            name: "Rectangle",
            parents: [
                Named("Polygon"),
            ],
            annotations: (),
        ),

        Declared(
            name: "Rhombus",
            parents: [
                Named("Polygon"),
            ],
            annotations: (),
        ),

        Defined(
            name: "Square",
            definition: And([
                Named("Rectangle"),
                Named("Rhombus"),
            ]),
            annotations: (),
        ),

        Declared(
            name: "Circle",
            parents: [
                Named("Figure"),
            ],
            annotations: (),
        ),
    ],

    relations: [],

    axioms: [
        (
            name: "polygon_circle_disjoint",
            statement: Disjoint(
                left: Named("Polygon"),
                right: Named("Circle"),
            ),
            provenance: (
                kind: Definition,
            ),
        ),
    ],

    lexemes: [],
    contexts: [],
    claims: [],
)
```

The exact RON representation may be made more compact through Serde attributes, but readability should be prioritized over clever serialization tricks.

---

# 7. Canonicalization

LLM output should not become the final source text verbatim.

After successful parsing and admission:

1. Deserialize the candidate.
2. Validate it.
3. Normalize the semantic structures.
4. Sort declarations deterministically.
5. Serialize it through Know’s canonical RON writer.
6. Store the canonical result as the accepted `.know` file.

This means accepted files have stable formatting regardless of which model generated them.

Provide:

```text
know canonicalize geometry.know
```

Canonicalization should include:

* Deterministic declaration ordering
* Deterministic map and set ordering
* Stable enum representation
* Consistent indentation
* Trailing commas
* Stable line endings
* Optional generated headers
* No preservation of arbitrary LLM formatting

Semantic identity must not depend on declaration order.

---

# 8. Project manifest

Each knowledge package should have a `Know.toml` manifest:

```toml
[package]
name = "elementary-geometry"
version = "0.1.0"
schema = 1

[modules]
files = [
    "concepts.know",
    "lexicon.know",
    "contexts.know",
]

[validation]
deny_duplicate_definitions = true
deny_recursive_definitions = true
deny_inconsistent_entities = true
allow_unsatisfiable_concepts = false

[admission]
policy = "automatic-strict-v1"
required_independent_runs = 3
require_counterexample_pass = true
escalate_on_existing_definition_change = true
```

The manifest handles project configuration. `.know` files should not contain imports, executable module-loading behavior, or environment-dependent configuration.

---

# 9. Semantic commitments

Create `docs/semantics.md` before implementing the reasoner.

## 9.1 Open-world reasoning

Missing knowledge is not false.

For knowledge body (K) and proposition (P):

```text
Entailed:
    K proves P

Contradicted:
    K proves not P

Unknown:
    K proves neither P nor not P
```

## 9.2 Explicit ambiguity

`Ambiguous` means the proposition cannot yet be uniquely formed because multiple valid sense bindings remain.

It is distinct from `Unknown`.

```text
Ambiguous:
    Know does not know which proposition is intended.

Unknown:
    The proposition is known, but its truth is unresolved.
```

## 9.3 Definitions versus axioms

A definition introduces a concept through necessary and sufficient conditions.

An axiom adds a constraint to the knowledge body.

A claim records a provenance-bearing assertion.

A lexical sense connects a word form to a concept.

Do not collapse these into generic graph edges.

## 9.4 Unsatisfiable concepts

A named concept can be impossible without making every other part of the module unusable.

Distinguish:

```text
unsatisfiable concept
inconsistent entity
inconsistent active knowledge body
```

## 9.5 No silent vacuous confirmation

A universal statement may be formally true because its subject concept is unsatisfiable.

Explanations should mark this:

```rust
pub struct Entailment {
    pub derivation: Derivation,
    pub vacuous: bool,
}
```

## 9.6 Monotonic accepted knowledge

Accepted definitions should not silently change meaning.

A changed accepted definition creates a new knowledge version and invalidates or recomputes dependent conclusions.

---

# 10. Validation pipeline

Implement validation as explicit passes:

```text
1. RON parsing
2. Serde schema validation
3. Schema-version validation
4. File and module collection
5. Symbol collection
6. Duplicate detection
7. Name resolution
8. Structural type checking
9. Definition dependency graph
10. Definition-cycle analysis
11. Expression normalization
12. Taxonomic closure
13. Disjointness propagation
14. Concept satisfiability analysis
15. Entity consistency analysis
16. Active knowledge consistency result
17. Canonical serialization
```

Each pass should return structured diagnostics.

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
    pub file: Option<PathBuf>,
    pub path: Option<DataPath>,
    pub related: Vec<RelatedDiagnostic>,
}
```

Because RON does not naturally provide language-level source spans for every deserialized field, preserve useful structural paths:

```text
concepts[5].definition.And[1]
axioms[0].statement.Disjoint.right
lexemes[2].senses[1].concept
```

Where practical, retain parser spans or line and column information from the RON syntax layer.

Stable diagnostic codes should include:

```text
K001 malformed RON
K002 unsupported schema version
K003 duplicate symbol
K004 unresolved concept
K005 duplicate definition
K006 recursive definition
K007 invalid expression
K008 unsatisfiable concept
K009 inconsistent entity
K010 invalid sense binding
K011 ambiguous sense
K012 admission regression
```

---

# 11. Native Rust reasoner

Implement a native reasoner over the restricted V1 algebra.

Support:

* Named concept inheritance
* Transitive subclass closure
* Equivalence
* Intersections
* Limited unions where the semantics are implemented soundly
* Explicit negation
* Disjointness
* Named entity membership
* Explicit negative membership
* Concept satisfiability
* Entity consistency
* Query explanations

Prefer sound incompleteness over unsound inference.

When Know cannot validly derive a proposition or its negation, return `Unknown`.

## Verdict structure

```rust
pub enum Verdict {
    Entailed(Entailment),
    Contradicted(Contradiction),
    Unknown(UnknownReason),
    Ambiguous(Ambiguity),
    IllTyped(Vec<Diagnostic>),
    Inconsistent(Conflict),
}
```

## Derivations

Every entailed or contradicted result must include a derivation:

```rust
pub enum Derivation {
    Declared {
        source: StatementId,
    },

    DefinitionExpansion {
        definition: DefinitionId,
        result: Box<Derivation>,
    },

    Transitive {
        first: Box<Derivation>,
        second: Box<Derivation>,
    },

    Disjointness {
        left: Box<Derivation>,
        right: Box<Derivation>,
        axiom: StatementId,
    },
}
```

Explanation generation is a core feature, not optional debugging output.

---

# 12. Rust builder API

Even though `.know` files are canonical artifacts, provide a typed Rust builder API for:

* Engine tests
* Programmatic generation
* Fuzzing
* Internal tools
* Admission pipeline construction
* Schema migration tests

Example:

```rust
let mut module = KnowledgeModuleBuilder::new("geometry");

let figure = module.primitive("Figure")?;
let polygon = module.declare("Polygon", [figure])?;
let rectangle = module.declare("Rectangle", [polygon])?;
let rhombus = module.declare("Rhombus", [polygon])?;
let circle = module.declare("Circle", [figure])?;

let square = module.define(
    "Square",
    ConceptExpr::named(rectangle)
        .and(ConceptExpr::named(rhombus)),
)?;

module.disjoint("polygon_circle_disjoint", polygon, circle)?;

let source = module.finish_source()?;
```

The builder API and RON deserializer must both produce the same source schema and validated IR.

The builder API must not become a second semantic implementation.

---

# 13. LLM proposal format

The LLM may produce:

* JSON structured output
* RON
* Tool-call arguments
* Direct Rust-side candidate structures

Do not require the model to perfectly format final `.know` files.

The preferred workflow is:

```text
LLM structured candidate
        ↓
deserialize into CandidatePatch
        ↓
validate and refine
        ↓
apply to isolated knowledge snapshot
        ↓
run semantic and regression checks
        ↓
admit automatically
        ↓
serialize canonical RON as .know
```

Define an explicit candidate patch:

```rust
pub struct CandidatePatch {
    pub base_version: KnowledgeVersion,
    pub additions: Vec<KnowledgeItemSource>,
    pub removals: Vec<KnowledgeItemRef>,
    pub replacements: Vec<ReplacementSource>,
    pub generation: GenerationRecord,
}
```

The LLM should propose transactions rather than rewriting arbitrary complete files whenever possible.

---

# 14. Automated admission pipeline

The admission process should be automated first.

For each candidate definition:

1. Parse the structured output.
2. Confirm every referenced concept exists or is included in the same candidate patch.
3. Classify each item as a definition, axiom, claim, lexical binding, or context preference.
4. Normalize the candidate.
5. Detect circularity.
6. Check satisfiability.
7. Check compatibility with accepted knowledge.
8. Identify changed prior verdicts.
9. Ask independent model runs for alternative definitions.
10. Ask adversarial model runs for counterexamples.
11. Compare candidate stability across runs.
12. Apply the configured admission policy.
13. Accept, reject, or escalate.
14. Record the complete audit event.
15. Canonicalize accepted output into `.know` files.

Automatic escalation should occur for cases such as:

* Multiple coherent but incompatible definitions
* Changes to accepted primitives
* Changes that invalidate many prior entailments
* Unresolved word senses
* Repeated disagreement among extraction runs
* Dependence on undefined or circular concepts
* A proposed empirical statement without required provenance
* A proposal that creates unsatisfiable concepts unexpectedly

---

# 15. Audit model

Every accepted item should be traceable.

```rust
pub struct AdmissionRecord {
    pub id: AdmissionId,
    pub knowledge_version: KnowledgeVersion,
    pub candidate_hash: ContentHash,
    pub generated_by: ModelIdentity,
    pub generation_inputs: Vec<GenerationInputRef>,
    pub independent_runs: Vec<ModelRunRef>,
    pub counterexample_runs: Vec<ModelRunRef>,
    pub validation_results: Vec<CheckResult>,
    pub changed_verdicts: Vec<VerdictDiff>,
    pub admission_policy: AdmissionPolicyId,
    pub outcome: AdmissionOutcome,
    pub human_interventions: Vec<HumanInterventionRef>,
}
```

The canonical `.know` files should remain readable and uncluttered. Store verbose audit information separately in an append-only log or database.

The engine should make it easy to answer:

```text
Why was Square defined this way?
Which model generated this definition?
Which alternatives were rejected?
Which concepts depend on this definition?
Which verdicts would change if it were removed?
Has a human ever modified it?
```

---

# 16. Geometry MVP

Use elementary geometric categories as the first domain.

## Concepts

```text
SpatialObject
Figure
ClosedFigure
Polygon
Quadrilateral
Rectangle
Rhombus
Square
Triangle
Circle
EqualSided
RightAngled
Red
Large
```

## Core knowledge

```text
Figure is a SpatialObject.
ClosedFigure is a Figure.
Polygon is a ClosedFigure.
Quadrilateral is a Polygon.
Rectangle is a Quadrilateral.
Rhombus is a Quadrilateral.
Square is equivalent to Rectangle and Rhombus.
Triangle is a Polygon.
Circle is a ClosedFigure.
Polygon is disjoint from Circle.
```

## Lexical senses

```text
"square" noun geometry → Square
"square" noun public-space → PublicSquare
"square" verb arithmetic → SquareOperation
"rectangle" noun geometry → Rectangle
"circle" noun geometry → Circle
"large" adjective relative-size → Large
```

## Required results

```text
Square subclass Rectangle
→ Entailed

Square subclass Polygon
→ Entailed

Square overlaps Circle
→ Contradicted

Red and Square subclass Large
→ Unknown

Square and Circle is satisfiable
→ Contradicted

An entity explicitly classified as Square and Circle
→ Inconsistent

The word "square" without context
→ Ambiguous

The word "square" in elementary geometry
→ resolves to the geometric sense
```

---

# 17. Correctness strategy

## Unit tests

Test each inference rule independently.

## Table-driven semantic tests

Create at least 100 hand-authored semantic cases.

## Property-based tests

Test invariants such as:

* Subclass closure is transitive.
* Equivalence is symmetric.
* Disjointness is symmetric.
* Normalization is idempotent.
* Declaration order does not affect results.
* Serialization round trips preserve semantics.
* A consistent knowledge body cannot entail both a proposition and its negation.
* Canonicalization is stable across repeated runs.

## Exhaustive finite-model oracle

For small generated knowledge bodies, enumerate bounded interpretations and determine:

```text
Entailed:
    every valid model satisfies P

Contradicted:
    every valid model satisfies not P

Unknown:
    some valid models satisfy P and others do not
```

Compare the optimized reasoner against this reference implementation.

## Mutation testing

Deliberately introduce:

* Contradictory parent concepts
* Duplicate definitions
* Circular definitions
* Undeclared names
* Incorrect sense bindings
* Unsatisfiable named concepts
* Contradictory entity classifications
* Schema-version mismatches

## Fuzzing

Fuzz:

* RON parsing
* Deserialization
* Schema migration
* Expression normalization
* Dependency graphs
* Query evaluation
* Canonical serialization
* Explanation generation

Malformed external input must never panic the engine.

---

# 18. Schema evolution

Every `.know` module must declare a schema version.

```rust
schema_version: 1
```

Implement migrations in Rust:

```rust
pub trait SchemaMigration {
    fn from_version(&self) -> u32;
    fn to_version(&self) -> u32;
    fn migrate(&self, value: ron::Value) -> Result<ron::Value, MigrationError>;
}
```

Do not require users or LLMs to manually update every old file when the Rust schema evolves.

Provide:

```text
know migrate knowledge/geometry
```

Accepted knowledge versions should remain reproducible with their original engine and schema versions.

---

# 19. Deferred features

Do not implement during the groundwork phase:

* A custom `.know` grammar
* A `.know` procedural macro
* A standalone language server for custom syntax
* Lean integration
* OWL export
* SAT or SMT integration
* Natural-language parsing
* Embedding-based sense resolution
* General LLM extraction
* Universal common-sense knowledge
* Fuzzy truth
* Probabilistic propositions
* Default reasoning
* Temporal reasoning
* Deontic reasoning
* Higher-order logic
* User-defined proof rules
* Self-modifying semantics
* Arbitrary relation cardinality
* GUI tooling

Create clean interfaces, but do not build speculative abstractions for every future backend.

---

# 20. Implementation milestones

## Milestone 1: Semantic specification

Deliver:

* `docs/semantics.md`
* Exact definitions of all verdicts
* Exact distinction among definitions, axioms, claims, and lexical bindings
* Exact V1 concept-expression semantics
* Explicit non-feature list

## Milestone 2: Source schema

Deliver:

* Serde-compatible Rust structs and enums
* RON deserialization
* Schema-version checks
* Basic `.know` fixtures
* Round-trip tests

## Milestone 3: Validated IR

Deliver:

* Opaque identifiers
* Symbol interning
* Module resolution
* Source-to-IR compilation
* Duplicate and unresolved-name diagnostics
* Deterministic canonicalization

## Milestone 4: Structural validator

Deliver:

* Expression validation
* Definition-cycle detection
* Duplicate-definition detection
* Relation type validation
* Lexical sense validation
* Structured diagnostics

## Milestone 5: Taxonomic reasoner

Deliver:

* Subclass closure
* Equivalence
* Intersections
* Disjointness
* Named-concept satisfiability
* Query verdicts
* Explanation trees

## Milestone 6: Entities and explicit negation

Deliver:

* Entity declarations
* Positive membership
* Explicit negative membership
* Contradictory membership detection

## Milestone 7: Lexical layer

Deliver:

* Lexemes
* Multiple senses
* Context preference
* Ambiguity detection
* Explicit interpretation requests

## Milestone 8: Correctness infrastructure

Deliver:

* At least 100 semantic cases
* Property-based tests
* Finite-model reference oracle
* Mutation tests
* Fuzz targets

## Milestone 9: Automated admission skeleton

Deliver:

* `CandidatePatch`
* Isolated candidate evaluation
* Regression diffing
* Admission policies
* Audit records
* Mocked independent and adversarial model runs

## Milestone 10: CLI

Deliver:

```text
know validate
know check
know query
know explain
know canonicalize
know migrate
know audit
```

---

# 21. Definition of done

The groundwork is complete when:

1. `.know` files use RON and deserialize directly into typed Rust source structures.
2. The project contains no custom knowledge-language parser.
3. The Rust source schema is distinct from the resolved reasoner IR.
4. Words, senses, concepts, definitions, axioms, and claims are separate types.
5. The geometry knowledge body can be loaded from `.know` files.
6. The engine returns all six verdict categories.
7. Every entailed or contradicted verdict includes an explanation.
8. Missing information produces `Unknown`, not false.
9. Unresolved senses produce `Ambiguous`.
10. Unsatisfiable concepts are detected.
11. Contradictory entity classifications are detected.
12. Accepted files serialize into deterministic canonical RON.
13. Schema migrations are possible without manually rewriting every file.
14. The optimized reasoner agrees with the finite-model oracle within the supported fragment.
15. Candidate LLM knowledge can be evaluated in isolation.
16. Admission can occur automatically under an explicit policy.
17. Every automated admission produces a complete audit record.
18. Human corrections can supersede accepted knowledge without erasing history.
19. No semantic rule exists only in the CLI, serializer, or admission layer.
20. The repository clearly documents what Know cannot yet represent.

---

# 22. Build-agent constraints

* Treat RON as serialization, not as Know’s semantic language.
* Do not add custom syntax unless explicitly requested later.
* Keep Rust types authoritative.
* Prefer explicit enums over loosely typed maps or generic triples.
* Keep source schema and validated IR separate.
* Reject invalid states before they reach the reasoner.
* Return `Unknown` rather than inventing unsupported conclusions.
* Make every accepted relationship explainable.
* Keep audit metadata outside the readable knowledge files where possible.
* Ensure declaration order has no semantic effect.
* Canonicalize all accepted LLM output before storing it.
* Do not require routine human approval for automated admission.
* Escalate only when the configured admission policy cannot resolve the proposal safely.
* Add no logical feature without defining its semantics and testing its interaction with existing features.
* Optimize for a small, trustworthy semantic core rather than broad expressiveness.
