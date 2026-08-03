# Know Framework Groundwork Plan

## Objective

Build the foundational Rust framework for **Know**, an executable system of discrete word senses, formal concept definitions, and mechanically checkable abstract knowledge.

Know must eventually allow an AI to:

1. Propose a word-sense interpretation.
2. Expose the selected sense bindings.
3. Translate a statement into a formal proposition.
4. Check that proposition against an accepted knowledge body.
5. Return one of:

```rust
Entailed
Contradicted
Unknown
Ambiguous
IllTyped
Inconsistent
```

This phase should not attempt to model all of English. Its purpose is to establish a small, correct, extensible semantic kernel that future dictionaries and LLM extraction workflows can safely target.

---

# 1. Architectural direction

Implement Know as a Rust workspace with a frontend-independent semantic engine.

The initial authoring interface should be an ordinary typed Rust API. Do not build a standalone `.know` parser yet. Do not encode knowledge through Rust traits and expect `rustc` to act as the ontology reasoner.

The architecture should be:

```text
Rust builder API
        ↓
Canonical Know IR
        ↓
Structural validator
        ↓
Semantic reasoner
        ↓
Verdict and explanation
```

Future frontends should compile into the same IR:

```text
Rust builder API ─────┐
Rust know! macro ─────┼──→ Know IR
.know files ──────────┤
LLM JSON protocol ────┘
```

The Know IR and reasoner must not depend on any particular frontend.

---

# 2. Cargo workspace

Create a workspace with the following crates:

```text
know/
├── crates/
│   ├── know-core/
│   ├── know-engine/
│   ├── know-cli/
├── tests/
│   └── fixtures/
├── examples/
│   └── geometry/
├── docs/
│   ├── semantics.md
│   ├── architecture.md
│   └── roadmap.md
└── Cargo.toml
```

Do not create `know-macros` or a standalone parser during the first phase. Add those only after the IR and semantics stabilize.

## `know-core`

Owns the canonical data model:

* IDs
* Symbols
* Concepts
* Concept expressions
* Relations
* Definitions
* Axioms
* Lexemes
* Senses
* Propositions
* Modules
* Provenance
* Diagnostics
* Verdict types

This crate must contain no reasoning implementation and minimal I/O.

## `know-engine`

Owns:

* Name resolution
* Structural validation
* Type checking
* Dependency analysis
* Normalization
* Inference
* Consistency checking
* Explanation generation
* Query evaluation

## `know-cli`

Initially provides:

```text
know check
know explain
know dump
know validate
```

The CLI may load a serialized Know IR fixture. It does not need a custom source language yet.

## `tests/fixtures`

Contains:

* Test knowledge-base builders
* Geometry fixtures
* Assertion helpers
* Random-model generators
* Exhaustive finite-model oracle
* Mutation-test utilities

---

# 3. Semantic commitments

Document the V1 semantics before implementing the reasoner.

Create `docs/semantics.md` and define every construct mathematically and operationally.

## 3.1 Open-world reasoning

Missing knowledge must not be treated as false.

Given no relationship between `RedSquare` and `Large`, the result must be:

```text
Unknown
```

not:

```text
Contradicted
```

## 3.2 Monotonic accepted knowledge

Adding accepted knowledge may turn `Unknown` into `Entailed` or `Contradicted`.

It must not silently remove previously accepted knowledge or reinterpret existing symbols.

Changes to accepted definitions should create new semantic versions rather than mutate historical meaning invisibly.

Full versioning may be implemented later, but the data model must not prevent it.

## 3.3 Definitions versus axioms

Maintain a strict distinction between:

```text
Definition
Axiom
Claim
LexicalBinding
```

A definition introduces or completely characterizes a concept:

```text
Square ≡ Rectangle ∩ EqualSided
```

An axiom constrains concepts that may already exist:

```text
Polygon disjoint Circle
```

A claim describes something asserted about the world and must carry provenance:

```text
BankA instance_of RegulatedInstitution
```

A lexical binding maps a word sense to a concept:

```text
"square"::noun::geometry → Square
```

Do not store these as interchangeable generic triples.

## 3.4 Explicit negation

Know must distinguish:

```text
not known to be A
known not to be A
```

The first produces `Unknown`. The second may produce `Contradicted`.

## 3.5 Concept satisfiability

A concept can be declared but logically impossible.

Example:

```text
ImpossibleSquare ⊆ Polygon
ImpossibleSquare ⊆ Circle
Polygon disjoint Circle
```

The engine must report `ImpossibleSquare` as unsatisfiable.

An unsatisfiable named concept should not necessarily make the entire module globally inconsistent. The diagnostics must distinguish:

```text
unsatisfiable concept
inconsistent assertion set
globally inconsistent module
```

## 3.6 No silent vacuous truth

For universal statements, track whether the subject concept is satisfiable.

A proposition may be formally entailed only because its subject is impossible. Explanations must expose this.

Consider adding a warning such as:

```rust
Entailed {
    vacuous: true,
    ...
}
```

Do not silently present vacuous entailment as ordinary semantic confirmation.

---

# 4. Canonical Rust data model

Use opaque IDs internally rather than strings throughout the engine.

```rust
pub struct ConceptId(u32);
pub struct RelationId(u32);
pub struct EntityId(u32);
pub struct LexemeId(u32);
pub struct SenseId(u32);
pub struct ModuleId(u32);
```

Names should be interned and resolved through symbol tables.

## 4.1 Concept expressions

Start with this restricted concept algebra:

```rust
pub enum ConceptExpr {
    Named(ConceptId),
    And(Vec<ConceptExpr>),
    Or(Vec<ConceptExpr>),
    Not(Box<ConceptExpr>),
}
```

Normalize expressions during validation:

* Flatten nested conjunctions.
* Flatten nested disjunctions.
* Remove duplicates.
* Sort operands deterministically.
* Eliminate double negation.
* Detect direct contradictions such as `A & !A`.
* Detect trivial expressions where possible.

Do not add quantifiers, cardinality, recursive definitions, arithmetic, temporal logic, fuzzy membership, or probabilistic truth in V1.

## 4.2 Statements

Represent semantic statements explicitly:

```rust
pub enum Statement {
    Subclass {
        child: ConceptExpr,
        parent: ConceptExpr,
    },
    Equivalent {
        left: ConceptExpr,
        right: ConceptExpr,
    },
    Disjoint {
        left: ConceptExpr,
        right: ConceptExpr,
    },
    InstanceOf {
        entity: EntityId,
        concept: ConceptExpr,
    },
    NotInstanceOf {
        entity: EntityId,
        concept: ConceptExpr,
    },
}
```

Definitions and axioms should wrap statements rather than being represented only by comments or metadata.

```rust
pub struct Definition {
    pub concept: ConceptId,
    pub expression: ConceptExpr,
    pub provenance: Provenance,
}

pub struct Axiom {
    pub statement: Statement,
    pub provenance: Provenance,
}
```

## 4.3 Lexical model

Represent the word/sense/concept distinction from the beginning:

```rust
pub struct Lexeme {
    pub id: LexemeId,
    pub written_form: String,
    pub language: LanguageTag,
    pub senses: Vec<SenseId>,
}

pub struct Sense {
    pub id: SenseId,
    pub lexeme: LexemeId,
    pub label: QualifiedName,
    pub concept: ConceptId,
    pub context: ContextId,
    pub status: SenseStatus,
    pub provenance: Provenance,
}
```

A word must never map directly to one concept without passing through a sense.

## 4.4 Candidate and accepted knowledge

The system should distinguish proposed LLM output from accepted knowledge:

```rust
pub enum AdmissionStatus {
    Candidate,
    Accepted,
    Rejected,
    Superseded,
}
```

Candidate statements must not participate in trusted inference unless explicitly evaluated in an isolated candidate context.

---

# 5. Builder API

Create a safe but ergonomic Rust builder API.

Target usage:

```rust
let mut kb = KnowledgeBaseBuilder::new("geometry");

let figure = kb.concept("Figure")?;
let polygon = kb.concept("Polygon")?;
let rectangle = kb.concept("Rectangle")?;
let equal_sided = kb.concept("EqualSided")?;
let square = kb.concept("Square")?;
let circle = kb.concept("Circle")?;

kb.subclass(polygon, figure)?;
kb.subclass(rectangle, polygon)?;

kb.define(
    square,
    ConceptExpr::named(rectangle).and(equal_sided),
)?;

kb.disjoint(polygon, circle)?;

let knowledge = kb.build()?;
```

The builder should reject local construction errors immediately:

* Duplicate names within a namespace
* References to undeclared symbols
* Duplicate complete definitions
* Direct self-definition
* Invalid empty conjunctions or disjunctions
* Conflicting entity declarations
* Invalid lexical sense bindings

The builder may collect multiple diagnostics before returning rather than stopping at the first error.

Do not expose raw vectors or allow callers to construct invalid internal IDs.

---

# 6. Validation pipeline

Implement validation as explicit passes.

```text
1. Symbol collection
2. Name resolution
3. Structural validation
4. Dependency graph construction
5. Definition-cycle analysis
6. Expression normalization
7. Taxonomic closure
8. Disjointness propagation
9. Concept satisfiability analysis
10. Entity consistency analysis
11. Module consistency result
```

Each pass should produce structured diagnostics with:

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
    pub primary_source: Option<SourceSpan>,
    pub related_sources: Vec<RelatedDiagnostic>,
}
```

Assign stable diagnostic codes from the beginning:

```text
K001 duplicate symbol
K002 unresolved concept
K003 recursive definition
K004 conflicting definitions
K005 unsatisfiable concept
K006 inconsistent entity
K007 invalid lexical binding
K008 ambiguous lexical reference
```

Stable codes will matter later for editor tooling, LLM repair loops, and automated ingestion.

---

# 7. Native V1 reasoner

Implement a small native Rust reasoner rather than integrating OWL, Lean, or SMT during the first phase.

The V1 reasoner should support:

* Named concept inheritance
* Transitive subclass closure
* Equivalence
* Conjunction
* Disjunction where soundly decidable in the selected fragment
* Explicit negation
* Disjointness
* Named entity membership
* Explicit negative membership
* Unsatisfiable-concept detection
* Contradictory entity-membership detection

Prefer correctness and explicit incompleteness over clever but unsound inference.

If the engine cannot prove a proposition or its negation, return `Unknown`.

Do not infer a result merely because it appears intuitively obvious.

## Verdict API

Use a verdict structure that carries evidence:

```rust
pub enum Verdict {
    Entailed(Proof),
    Contradicted(Proof),
    Unknown(UnknownReason),
    Ambiguous(Ambiguity),
    IllTyped(Vec<Diagnostic>),
    Inconsistent(Conflict),
}
```

A `Proof` need not be a general theorem-prover proof object. For V1, it may be a derivation tree over a small set of rules:

```rust
pub enum Derivation {
    Declared { source: StatementId },
    Transitive {
        left: Box<Derivation>,
        right: Box<Derivation>,
    },
    EquivalentExpansion {
        definition: DefinitionId,
        child: Box<Derivation>,
    },
    Disjointness {
        left: Box<Derivation>,
        right: Box<Derivation>,
        disjoint_axiom: StatementId,
    },
}
```

Every `Entailed` or `Contradicted` result must be explainable.

---

# 8. Reference oracle and correctness strategy

The reasoner must not be trusted solely because its unit tests pass.

Build several layers of verification.

## 8.1 Unit tests

Test every inference rule independently.

Examples:

* Direct subclass entailment
* Transitive subclass entailment
* Equivalence in both directions
* Disjointness symmetry
* Disjointness inheritance
* Unknown under absent information
* Explicit negation
* Unsatisfiable intersection
* Contradictory entity classification

## 8.2 Table-driven semantic tests

Create declarative test fixtures:

```rust
SemanticCase {
    knowledge: ...,
    proposition: ...,
    expected: VerdictKind::Entailed,
}
```

Start with at least 100 hand-authored cases.

## 8.3 Property-based tests

Generate random small ontologies and assert invariants such as:

* Subclass entailment is reflexive where the semantics require it.
* Subclass entailment is transitive.
* Equivalence is symmetric.
* Disjointness is symmetric.
* A consistent theory cannot entail both `P` and `not P`.
* Normalization is idempotent.
* Serialization round trips preserve semantics.
* Declaration order does not affect verdicts.

## 8.4 Exhaustive finite-model oracle

For very small knowledge bases, enumerate all possible finite interpretations over a bounded domain.

Use the enumerator as an independent oracle:

```text
P is entailed if every valid model satisfies P.
P is contradicted if every valid model satisfies not P.
Otherwise P is unknown.
```

Compare the optimized native reasoner against this oracle for thousands of generated small cases.

The finite oracle does not define the final unbounded semantics, but it provides a powerful independent check for the initial fragment.

## 8.5 Differential testing

Once the semantic fragment has a correct mapping to another solver, optionally compare results against an external backend.

This is not required for the first implementation milestone. Design the engine so a future backend can be added without changing the public IR.

## 8.6 Fuzzing

Fuzz:

* Builder operations
* Serialization input
* Expression normalization
* Dependency graphs
* Query evaluation
* Explanation generation

The engine must never panic on malformed external input.

---

# 9. Geometry reference module

Create a deliberately narrow geometry module as the canonical acceptance suite.

Concepts should include:

```text
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
Red
Large
```

Suggested knowledge:

```text
Polygon subclass Figure
Quadrilateral subclass Polygon
Rectangle subclass Quadrilateral
Rhombus subclass Quadrilateral
Square equivalent Rectangle and Rhombus
Triangle subclass Polygon
Circle subclass ClosedFigure
Circle subclass Figure
Polygon disjoint Circle
```

Lexical senses:

```text
"square"::noun::geometry → Square
"square"::verb::arithmetic → SquareOperation
"circle"::noun::geometry → Circle
"large"::adjective::relative_size → Large
```

Required verdicts:

```text
Square subclass Rectangle
→ Entailed

Square subclass Polygon
→ Entailed

Square overlaps Circle
→ Contradicted

Red and Square subclass Large
→ Unknown

Square equivalent Rectangle
→ Unknown or Contradicted only if the accepted theory proves non-equivalence

Square and Circle is satisfiable
→ Contradicted

An entity explicitly classified as Square and Circle
→ Inconsistent entity classification

The word "square" without context
→ Ambiguous

"square" under elementary-geometry context
→ resolves to Square
```

Add mutation tests that deliberately introduce:

* A contradictory parent
* A duplicate definition
* A circular definition
* An undeclared concept
* A conflicting sense binding
* An unsatisfiable named concept

---

# 10. Lexical interpretation boundary

Do not attempt automated natural-language parsing yet.

Implement an explicit interpretation request structure:

```rust
pub struct InterpretationRequest {
    pub source_text: String,
    pub context: ContextId,
    pub bindings: Vec<SenseBinding>,
    pub proposition: Proposition,
}
```

The engine must validate:

* Every referenced word occurrence has an explicit sense binding when required.
* Every sense exists.
* Every sense is permitted in the selected context.
* The formal proposition uses the concepts associated with the supplied bindings.
* No undeclared concept was silently invented.
* The proposition is structurally and semantically well typed.

This preserves the eventual AI-facing contract without requiring an LLM integration yet.

---

# 11. Persistence

Implement a stable serialized representation only after the in-memory IR works.

Requirements:

* Deterministic serialization
* Explicit schema version
* Stable qualified identifiers
* No serialization of ephemeral numeric IDs as semantic identity
* Full provenance preservation
* Round-trip tests
* Human-inspectable debug representation

Use a format such as JSON initially for debugging and tests. Do not treat the first serialized representation as the final public `.know` language.

Example:

```json
{
  "schema_version": 1,
  "module": "geometry",
  "concepts": [
    {
      "name": "Square",
      "definition": {
        "and": [
          {"named": "Rectangle"},
          {"named": "Rhombus"}
        ]
      }
    }
  ]
}
```

---

# 12. Deferred features

Do not implement the following during the groundwork phase:

* Standalone `.know` files
* Procedural macros
* Lean integration
* OWL export
* SAT or SMT integration
* Embedding generation
* LLM extraction
* Automatic definition acceptance
* Natural-language parsing
* Probability or confidence within logical truth
* Fuzzy concepts
* Defaults or non-monotonic reasoning
* Temporal reasoning
* Deontic reasoning
* Arbitrary relation cardinality
* Higher-order logic
* Universal common-sense knowledge
* Networked knowledge sharing
* GUI or editor extension

Create interfaces where appropriate, but do not build speculative abstractions for all deferred features.

---

# 13. Implementation order

## Milestone 1: Semantic specification

Deliver:

* `docs/semantics.md`
* `docs/architecture.md`
* Exact definition of each verdict
* Exact distinction among definitions, axioms, claims, and lexical bindings
* Explicit V1 feature and non-feature list

Do not start inference implementation until the semantics document is internally consistent.

## Milestone 2: Core IR

Deliver:

* Opaque identifiers
* Symbol tables
* Concept expressions
* Statements
* Definitions
* Axioms
* Lexemes and senses
* Contexts
* Provenance
* Structured diagnostics
* Deterministic debug formatting

All public types must have unit tests.

## Milestone 3: Builder and validation

Deliver:

* `KnowledgeBaseBuilder`
* Name resolution
* Duplicate detection
* Definition dependency graph
* Cycle detection
* Expression normalization
* Local validation
* Structured diagnostic reporting

## Milestone 4: Taxonomic reasoner

Deliver:

* Subclass closure
* Equivalence
* Disjointness
* Named-concept satisfiability
* Query evaluation
* Explanation trees

## Milestone 5: Entities and explicit negation

Deliver:

* Named entities
* Positive membership
* Negative membership
* Contradictory membership detection
* Entity-level explanations

## Milestone 6: Lexical layer

Deliver:

* Lexemes
* Multiple senses
* Context-restricted sense resolution
* Ambiguity results
* Interpretation request validation

## Milestone 7: Correctness infrastructure

Deliver:

* 100 or more semantic cases
* Property-based tests
* Finite-model reference oracle
* Differential checks between optimized reasoner and oracle
* Fuzz targets
* Mutation cases

## Milestone 8: CLI and serialized fixtures

Deliver:

```text
know validate geometry.json
know check geometry.json query.json
know explain geometry.json query.json
know dump geometry.json
```

## Milestone 9: Evaluate authoring frontend

Only after the previous milestones, evaluate whether to add:

```rust
know! {
    concept Square = Rectangle & Rhombus;
}
```

The macro must be a thin frontend over the existing IR. It must not contain separate semantic logic.

---

# 14. Definition of done

The groundwork phase is complete when all of the following are true:

1. Know can represent words, senses, concepts, definitions, axioms, and claims as distinct types.
2. The geometry module can be authored entirely through the Rust builder API.
3. The engine returns all six required verdict categories.
4. Every entailed or contradicted verdict includes an inspectable derivation.
5. Unknown is preserved under open-world semantics.
6. Ambiguous word senses remain unresolved until a context or binding selects one.
7. Unsatisfiable concepts are detected.
8. Contradictory entity classifications are detected.
9. The engine never treats candidate LLM output as accepted knowledge automatically.
10. The optimized reasoner agrees with the exhaustive finite-model oracle on generated cases within the supported fragment.
11. All public data structures serialize and deserialize deterministically.
12. The engine can be used without a custom source language.
13. No semantic rule exists only inside the CLI, builder, or a future macro frontend.
14. The repository clearly documents what the engine cannot yet express.
15. Adding a new logical operator requires compiler-visible updates to normalization, validation, reasoning, serialization, and tests.

---

# 15. Guiding principles for the build agent

* Prefer a small logic that is sound over a broad logic that is intuitive but underspecified.
* Return `Unknown` whenever the engine lacks a valid derivation.
* Keep syntax, data representation, and semantics separate.
* Do not confuse Rust type correctness with Know semantic correctness.
* Use Rust’s type system to make invalid internal states difficult to construct.
* Make every accepted semantic relationship inspectable and attributable.
* Preserve the distinction between a word, a sense, and a concept everywhere.
* Do not introduce a standalone language until the underlying semantic model has stabilized.
* Do not accept LLM output directly into the trusted knowledge body.
* Treat explanation generation as a core feature, not debugging output.
* Avoid hidden inference rules.
* Document every inference rule implemented by the engine.
* Add no expressive feature without tests proving its interaction with all existing features.
