# Know: Revised Architecture and Build Plan

## 1. Project definition

Know is a Rust-native system for building, validating, and reasoning over a cumulative body of discrete semantic knowledge.

Know does not attempt to define ordinary English words directly. Natural-language words are ambiguous shortcuts. Know instead creates **canonical concepts**, where each accepted concept has:

* one stable identity,
* one intended meaning,
* one formal logical representation,
* explicit relationships to other canonical concepts,
* provenance and version history.

English is treated as an input layer that must be resolved into canonical concepts before logical reasoning occurs.

The core pipeline is:

```text
Natural-language statement
        ↓
lexical and contextual resolution
        ↓
canonical concept bindings
        ↓
formal proposition
        ↓
OWL-aligned semantic validation
        ↓
reasoning
        ↓
Entailed / Contradicted / Unknown / Ambiguous / IllTyped / Inconsistent
```

Know should reuse the formal semantics developed by OWL and description logic while replacing OWL’s dated authoring formats, tooling assumptions, and knowledge-acquisition workflow.

---

# 2. Foundational principles

## 2.1 Words are not concepts

An English word may refer to several distinct concepts.

For example:

```text
"bank"
    → finance::bank
    → geography::river_bank
    → aviation::banking_maneuver
    → storage::reserved_collection
```

The word `"bank"` is not itself a formal concept.

It is a lexical form associated with several candidate concepts.

Know must therefore maintain a strict distinction between:

```text
LexicalForm
Concept
ConceptExpression
Entity
Proposition
```

A lexical form may map to many concepts.

Many lexical forms may map to the same concept.

A canonical concept ID must identify exactly one concept.

---

## 2.2 Polysemy is not logical OR

This is incorrect:

```text
Bank =
    FinancialBank
    OR
    RiverBank
    OR
    AviationBank
```

That treats lexical ambiguity as a logical union.

The correct representation is:

```text
Lexical form: "bank"

Candidate bindings:
    finance::bank
    geography::river_bank
    aviation::banking_maneuver
```

The lexical resolver must select or rank the intended concept before the proposition reaches the reasoner.

Logical `OR` remains available for concepts that are genuinely unions:

```text
MotorVehicle =
    Car
    OR
    Truck
    OR
    Motorcycle
```

The lexical layer and logical layer must never be conflated.

---

## 2.3 Similarity is not identity

Embedding similarity must not determine formal concept identity.

For example, `Cat` and `Dog` may be close in embedding space because they:

* are mammals,
* commonly bear the role of pet,
* share many physical characteristics,
* occur in similar sentences.

That does not mean they partially overlap as formal concepts.

Know should represent their relationship discretely:

```text
Cat SUBCLASS_OF Mammal
Dog SUBCLASS_OF Mammal
Cat DISJOINT_WITH Dog

Cat CAN_BEAR_ROLE Pet
Dog CAN_BEAR_ROLE Pet
```

The formal questions are:

```text
Are the concepts identical?
Is one a subtype of the other?
Are they disjoint?
Can one entity instantiate both?
Do they share a parent?
Do they share properties?
Are they merely lexically or statistically similar?
```

Embeddings may assist concept discovery and lexical resolution. They must not replace formal ontology.

---

## 2.4 Use OWL semantics without requiring OWL syntax

OWL should be treated as three separable things:

1. Formal semantics
2. An abstract ontology model
3. Concrete serializations such as RDF/XML, Turtle, Functional Syntax, and Manchester Syntax

Know should reuse the first and model the second in Rust.

Know should not require users or LLMs to author RDF, XML, Turtle, or Manchester Syntax.

The primary representation should be:

```text
.know files
    encoded as RON
        ↓
Serde
        ↓
Rust source structs
        ↓
validated ontology IR
```

RON is only the serialization format. Rust types define the ontology model. OWL-aligned semantics define what the ontology means.

---

# 3. Scope of Know Logic V1

Know should not attempt to support all of OWL 2 initially.

The first version should define a small, explicit, decidable fragment with documented semantics.

A suitable foundation is an ALC-like description logic.

## 3.1 Concept expressions

```rust
enum ConceptExpr {
    Named(ConceptId),

    And(Vec<ConceptExpr>),

    Or(Vec<ConceptExpr>),

    Not(Box<ConceptExpr>),

    Exists {
        relation: RelationId,
        filler: Box<ConceptExpr>,
    },

    ForAll {
        relation: RelationId,
        filler: Box<ConceptExpr>,
    },
}
```

These constructs correspond conceptually to:

```text
Named class
Intersection
Union
Complement
Existential relation restriction
Universal relation restriction
```

Examples:

```text
Square =
    Rectangle
    AND
    Rhombus
```

```text
MotorVehicle =
    Car
    OR
    Truck
    OR
    Motorcycle
```

```text
ChildlessPerson =
    Person
    AND
    NOT EXISTS has_child.Person
```

```text
Parent =
    Person
    AND
    EXISTS has_child.Person
```

```text
CatOnlyOwner =
    Person
    AND
    FOR_ALL owns.Cat
```

---

## 3.2 Axiom types

Know Logic V1 should support:

```rust
enum Axiom {
    SubclassOf {
        child: ConceptExpr,
        parent: ConceptExpr,
    },

    EquivalentClasses {
        classes: Vec<ConceptExpr>,
    },

    DisjointClasses {
        classes: Vec<ConceptExpr>,
    },

    ClassAssertion {
        entity: EntityId,
        class: ConceptExpr,
    },

    RelationAssertion {
        subject: EntityId,
        relation: RelationId,
        object: EntityId,
    },

    NegativeClassAssertion {
        entity: EntityId,
        class: ConceptExpr,
    },

    NegativeRelationAssertion {
        subject: EntityId,
        relation: RelationId,
        object: EntityId,
    },
}
```

V1 does not need to support every possible OWL feature.

The following should initially remain outside the supported fragment:

* property chains,
* arbitrary cardinality restrictions,
* datatype reasoning,
* keys,
* metaclasses,
* unrestricted rules,
* procedural functions,
* macros,
* loops,
* ontology mutation during reasoning.

Unsupported features must produce explicit errors rather than approximate results.

---

# 4. Negation and open-world reasoning

Know should adopt open-world semantics.

Failure to prove a proposition does not make the proposition false.

These are distinct:

```text
Unknown:
    Know cannot prove Cat(Fido)
    Know cannot prove NOT Cat(Fido)

Contradicted:
    Know can prove NOT Cat(Fido)

Entailed:
    Know can prove Cat(Fido)
```

Negation can enter the knowledge model through several mechanisms.

## 4.1 Concept complement

```text
NonCat = NOT Cat
```

This means the complement of `Cat` relative to the ontology’s interpretation domain.

## 4.2 Disjointness

```text
Cat DISJOINT_WITH Dog
```

If:

```text
Dog(Fido)
```

then:

```text
NOT Cat(Fido)
```

is derivable.

## 4.3 Explicit negative assertions

```text
NOT Cat(Fido)
```

This is stronger than merely lacking a positive assertion.

## 4.4 Negative relational assertions

```text
NOT owns(Alice, VehicleOne)
```

Again, this must be represented explicitly or derived logically.

Know must never use closed-world shortcuts such as:

```text
Not found in database
therefore false
```

unless a specific imported domain explicitly declares closed-world behavior. That should be a later and separately specified feature.

---

# 5. Canonical concept identity

## 5.1 Stable IDs

Each canonical concept should have a stable machine identity independent of its display name.

Example:

```text
know:animal:cat:001
know:animal:dog:001
know:finance:deposit_institution:001
know:geography:river_boundary:001
```

Human-readable labels may change without changing concept identity.

```rust
struct ConceptRecord {
    id: ConceptId,
    preferred_label: String,
    alternate_labels: Vec<String>,
    definition: Option<ConceptExpr>,
    grounding: Grounding,
    provenance: Provenance,
    status: ConceptStatus,
}
```

---

## 5.2 Definitions should reference canonical concepts

A concept definition must not contain unresolved English words.

Incorrect:

```text
Square =
    a shape with four equal sides
```

Correct:

```text
geometry::square =
    geometry::rectangle
    AND
    geometry::rhombus
```

The natural-language description may be retained as an annotation, but it is not the formal definition.

---

## 5.3 Primitive concepts

Not every concept can be reduced forever.

Know must allow explicitly declared primitives:

```rust
ConceptRecord {
    id: "know:physical:entity",
    definition: None,
    status: Primitive,
}
```

Primitive concepts should be accepted deliberately rather than appearing accidentally because extraction failed.

Their provenance and justification should remain visible.

---

## 5.4 Concept versioning

Definitions must not silently change underneath previously accepted proofs.

A materially changed meaning should create a new semantic version:

```text
finance::bank@1
finance::bank@2
```

Alternatively, concept records may have immutable internal IDs with explicit revision records.

The system must preserve:

* the old definition,
* the new definition,
* the reason for the change,
* affected propositions,
* invalidated or changed entailments.

---

# 6. Lexical resolution layer

The lexical layer maps language to canonical concepts.

```rust
struct LexicalForm {
    text: String,
    language: LanguageId,
    part_of_speech: Option<PartOfSpeech>,
    bindings: Vec<LexicalBinding>,
}

struct LexicalBinding {
    concept: ConceptId,
    usage_examples: Vec<String>,
    context_hints: Vec<ConceptId>,
    provenance: Provenance,
}
```

Example:

```text
LexicalForm(
    text: "bank",
    bindings: [
        (
            concept: "know:finance:deposit_institution:001",
            context_hints: [
                "know:finance:loan",
                "know:finance:deposit",
                "know:finance:account",
            ],
        ),
        (
            concept: "know:geography:river_boundary:001",
            context_hints: [
                "know:geography:river",
                "know:geography:shore",
                "know:geography:waterway",
            ],
        ),
    ],
)
```

The lexical resolver may use:

* surrounding words,
* grammatical role,
* domain context,
* embeddings,
* LLM interpretation,
* previously resolved discourse entities,
* explicit user context.

Its output must include:

```rust
struct ResolutionCandidate {
    concept: ConceptId,
    confidence: f32,
    evidence: Vec<ResolutionEvidence>,
}
```

Confidence is allowed at the lexical layer.

Confidence must not change the logical meaning of the selected canonical concept.

If no candidate clearly dominates, the result is:

```text
Ambiguous
```

The reasoner should never guess silently.

---

# 7. Rust architecture

Use a Cargo workspace with narrowly defined responsibilities.

```text
know/
    crates/
        know-core/
        know-lexicon/
        know-ontology/
        know-reasoner/
        know-admission/
        know-owl/
        know-cli/
        know-test-support/

    knowledge/
        core/
        geometry/
        biology/
        finance/
        geography/
```

## 7.1 `know-core`

Owns identifiers and common primitives:

* `ConceptId`
* `EntityId`
* `RelationId`
* `ModuleId`
* `Provenance`
* `SourceLocation`
* `SemanticVersion`
* shared diagnostics

No reasoning logic should live here.

---

## 7.2 `know-lexicon`

Owns:

* lexical forms,
* lexical bindings,
* contextual resolution,
* ambiguity reporting,
* embedding or LLM-assisted candidate generation,
* resolution evidence.

This crate must not decide ontology truth.

---

## 7.3 `know-ontology`

Owns:

* source structs deserialized from RON,
* concept expressions,
* axioms,
* ontology modules,
* normalization,
* semantic validation,
* canonical ontology IR.

This crate defines Know Logic’s abstract syntax.

---

## 7.4 `know-reasoner`

Owns:

* satisfiability,
* consistency checking,
* subclass entailment,
* equivalence,
* disjointness,
* entity classification,
* proposition evaluation,
* proof or explanation artifacts.

The reasoner must be sound and complete for the exact fragment Know claims to support.

Timeouts and unsupported expressions must never be returned as `Unknown`.

Use separate engine outcomes:

```rust
enum ReasoningOutcome<T> {
    Complete(T),
    Unsupported(UnsupportedFeature),
    ResourceLimit(ResourceLimit),
    InternalError(ReasoningError),
}
```

---

## 7.5 `know-admission`

Owns the automated knowledge-admission pipeline:

```text
proposal
    ↓
schema validation
    ↓
identifier resolution
    ↓
type validation
    ↓
logical consistency
    ↓
redundancy and equivalence checks
    ↓
regression tests
    ↓
admission policy
    ↓
accepted / rejected / deferred
```

Every decision must produce an audit record.

---

## 7.6 `know-owl`

Owns interoperability with established OWL semantics.

Initial responsibilities:

* export the supported Know fragment to OWL Functional Syntax,
* import the corresponding supported OWL subset,
* preserve stable mappings between Know IDs and OWL IRIs,
* run differential tests against a mature OWL reasoner,
* reject unsupported OWL constructs explicitly.

Know should not initially claim full OWL 2 compatibility.

The correct claim is:

> Know implements OWL-aligned semantics for a documented subset.

Full OWL representation in RON is structurally possible, but full compatibility would require implementing the semantics and validation rules for every supported OWL construct.

---

## 7.7 `know-cli`

Initial commands:

```text
know check <path>
know normalize <path>
know reason <query>
know classify <concept>
know explain <query>
know resolve <sentence>
know admit <proposal>
know export-owl <module>
know import-owl <ontology>
know diff-reasoner <module>
```

---

# 8. `.know` file representation

`.know` files should contain RON serialized directly into Rust source structs.

Example:

```rust
KnowledgeModule(
    id: "geometry",
    concepts: [
        Concept(
            id: "geometry::rectangle",
            label: "rectangle",
            definition: None,
            status: Primitive,
        ),
        Concept(
            id: "geometry::rhombus",
            label: "rhombus",
            definition: None,
            status: Primitive,
        ),
        Concept(
            id: "geometry::square",
            label: "square",
            definition: Some(
                And([
                    Named("geometry::rectangle"),
                    Named("geometry::rhombus"),
                ]),
            ),
            status: Defined,
        ),
    ],
    axioms: [
        DisjointClasses([
            Named("geometry::circle"),
            Named("geometry::polygon"),
        ]),
    ],
)
```

Do not create a custom parser for statements such as:

```text
concept Square = Rectangle & Rhombus;
```

unless a future authoring layer proves necessary.

The canonical storage format should remain structurally explicit and directly deserializable through Serde.

---

# 9. Reasoning semantics

## 9.1 Entailment

A proposition is `Entailed` when it is true in every model satisfying the accepted ontology.

## 9.2 Contradiction

A proposition is `Contradicted` when its negation is entailed.

## 9.3 Unknown

A proposition is `Unknown` when both the proposition and its negation remain possible in valid models of the ontology.

## 9.4 Inconsistent

The knowledge body is `Inconsistent` when no model satisfies all accepted axioms.

## 9.5 Ambiguous

A natural-language statement is `Ambiguous` when its lexical or structural interpretation has multiple unresolved formalizations.

This verdict belongs before logical reasoning.

## 9.6 IllTyped

A statement is `IllTyped` when it applies concepts or relations to incompatible categories.

Example:

```text
The number seven owns a river.
```

The statement may be syntactically valid but ontologically ill-typed if `owns` requires an agentive subject.

---

# 10. Reasoning implementation strategy

Do not begin by implementing the whole of OWL or an unrestricted description-logic reasoner.

Use staged implementation.

## Stage 1: ontology model and semantic contract

Implement:

* canonical concept IDs,
* named concepts,
* `AND`,
* `OR`,
* `NOT`,
* subclass axioms,
* equivalence,
* disjointness,
* entity class assertions,
* explicit negative assertions,
* consistency and entailment interfaces.

Document the model-theoretic meaning of each operation using OWL 2 Direct Semantics as the reference.

## Stage 2: Boolean reasoning backend

For the purely Boolean concept fragment, normalize expressions and compile reasoning questions to SAT or another complete Boolean decision procedure.

This stage must correctly distinguish:

* satisfiable,
* unsatisfiable,
* entailed,
* contradicted,
* unknown.

## Stage 3: relational description logic

Add:

* `EXISTS relation.Concept`,
* `FOR_ALL relation.Concept`,
* relation assertions.

At this stage, implement or integrate a complete procedure for the declared ALC-like fragment.

Do not approximate relational reasoning using ordinary graph traversal.

## Stage 4: OWL differential testing

Export Know ontologies to OWL and compare results with a mature reference reasoner.

For each test ontology, compare:

* consistency,
* satisfiable concepts,
* inferred subclass relationships,
* equivalence,
* disjointness,
* individual classification.

Any disagreement must be treated as either:

* a Know reasoner bug,
* an export bug,
* an unsupported semantic difference,
* or a misunderstood expected result.

## Stage 5: native optimization

Only after semantic correctness is established should the Rust reasoner be optimized through:

* expression interning,
* normalized concept DAGs,
* cached satisfiability checks,
* incremental reasoning,
* dependency-aware invalidation,
* parallel classification.

Correctness must precede performance.

---

# 11. Automated knowledge admission

LLMs should be allowed to propose new knowledge directly, but they must not bypass the admission pipeline.

A proposal should contain:

```rust
struct KnowledgeProposal {
    proposed_concepts: Vec<ConceptRecord>,
    proposed_axioms: Vec<Axiom>,
    lexical_bindings: Vec<LexicalForm>,
    source_evidence: Vec<SourceEvidence>,
    generated_by: GeneratorIdentity,
}
```

Admission should perform:

## 11.1 Structural validation

* valid RON,
* valid identifiers,
* no missing references,
* no duplicate immutable IDs,
* no unsupported constructors.

## 11.2 Lexical validation

* surface words are not treated as canonical concepts,
* polysemous lexical forms produce separate bindings,
* aliases do not silently create duplicate concepts.

## 11.3 Ontological validation

* definitions are well-typed,
* grounding categories are compatible,
* relation domain and range constraints are satisfied,
* no circular definitions without explicit support.

## 11.4 Logical validation

* ontology remains consistent,
* newly defined concepts are satisfiable,
* definitions do not accidentally collapse distinct concepts,
* new disjointness axioms do not invalidate accepted entities,
* equivalence claims are actually entailed.

## 11.5 Regression validation

Re-run relevant accepted queries and invariants.

Example invariants:

```text
Cat is disjoint with Dog.
Square is equivalent to Rectangle AND Rhombus.
No accepted concept is both satisfiable and explicitly impossible.
Previously entailed foundational axioms remain entailed.
```

## 11.6 Admission result

```rust
enum AdmissionDecision {
    Accepted,
    AcceptedWithWarnings,
    Rejected,
    DeferredForAmbiguity,
    DeferredForGrounding,
    ConflictsWithExistingKnowledge,
}
```

Every result should include machine-readable explanations.

Human review is reserved for conflicts, ambiguity, grounding disputes, or ontology redesign. Ordinary structurally and logically valid proposals should be admitted automatically.

---

# 12. Grounding categories

Know should not assume that every concept is the same kind of thing.

Add an explicit grounding classification:

```rust
enum Grounding {
    NaturalKind,
    StructuralDefinition,
    FunctionalKind,
    SocialKind,
    LegalKind,
    Role,
    Process,
    Event,
    MeasurementClass,
    EngineeredConcept,
    Primitive,
}
```

Examples:

```text
Cat          → NaturalKind
Square       → StructuralDefinition
Bank         → SocialKind or FunctionalKind
Pet          → Role
LegalMinor   → LegalKind
Running      → Process
Explosion    → Event
Large        → MeasurementClass
```

These categories may initially function as metadata and type constraints rather than full logical operators.

Their purpose is to prevent Know from treating a biological species, a legal status, and a geometric definition as ontologically interchangeable.

---

# 13. Proofs and explanations

Know should not return only a verdict.

It should return an explanation artifact.

Example:

```text
Query:
    Is Fido not a Cat?

Result:
    Entailed

Reason:
    1. Fido is asserted to be a Dog.
    2. Dog is disjoint with Cat.
    3. Therefore Fido cannot be a Cat.
```

Machine representation:

```rust
struct Explanation {
    conclusion: Proposition,
    supporting_axioms: Vec<AxiomId>,
    steps: Vec<InferenceStep>,
}
```

For `Unknown`, the explanation should state what is missing:

```text
Know contains no axiom establishing that every mammal is a cat,
and no axiom establishing that mammals are disjoint with cats.
Both Cat(Fido) and NOT Cat(Fido) remain logically possible.
```

`Unknown` should never mean merely that the engine gave up.

---

# 14. Testing requirements

## Unit tests

Test every concept constructor and axiom independently.

## Semantic tests

Create small ontologies with known outcomes for:

* conjunction,
* union,
* complement,
* equivalence,
* disjointness,
* explicit negation,
* open-world unknowns,
* existential restrictions,
* universal restrictions.

## Polysemy tests

Verify that:

```text
"bank" does not become a union concept.
```

Ensure lexical resolution returns distinct candidate concept IDs.

## Round-trip tests

Verify:

```text
RON
    → Rust source structs
    → normalized IR
    → RON
```

without semantic changes.

## Differential tests

Compare exported OWL ontologies with a reference OWL reasoner.

## Property-based tests

Generate random small ontologies and verify invariants such as:

```text
If A is equivalent to B, then A is a subclass of B.
If A and B are disjoint, A AND B is unsatisfiable.
If A is a subclass of B, A AND NOT B is unsatisfiable.
```

## Admission regression tests

Every previously discovered semantic bug should become a permanent admission test.

---

# 15. Explicit non-goals

Know V1 is not:

* a replacement for all of OWL,
* an RDF database,
* a natural-language model,
* an embedding database,
* a self-modifying programming language,
* a general theorem prover,
* a procedural rule engine,
* an attempt to formalize every aspect of English,
* a system that converts statistical similarity directly into truth.

Know should represent only what it can express discretely and reason over correctly.

Unsupported or irreducibly vague knowledge should remain unresolved rather than being forced into false precision.

---

# 16. Implementation phases

## Phase 1: canonical ontology model

Deliver:

* Rust identifiers,
* concept records,
* lexical forms,
* ontology modules,
* RON serialization,
* source validation,
* normalized IR.

No complex reasoning yet.

## Phase 2: Boolean concept semantics

Deliver:

* `AND`,
* `OR`,
* `NOT`,
* subclass,
* equivalence,
* disjointness,
* class assertions,
* negative class assertions,
* complete Boolean reasoning,
* formal verdicts.

## Phase 3: lexical resolution

Deliver:

* one-to-many word bindings,
* alias support,
* contextual candidate ranking,
* ambiguity verdicts,
* explicit resolution evidence,
* compiled propositions using canonical IDs.

## Phase 4: relational semantics

Deliver:

* relations,
* domain and range,
* existential restrictions,
* universal restrictions,
* relation assertions,
* ALC-like reasoning.

## Phase 5: OWL interoperability

Deliver:

* OWL Functional Syntax export,
* supported-subset import,
* stable IRI mapping,
* differential reasoning tests.

## Phase 6: admission pipeline

Deliver:

* LLM proposal schema,
* automated validation,
* consistency checks,
* regression checks,
* admission decisions,
* append-only audit log.

## Phase 7: cumulative knowledge packages

Begin with narrow, structurally clear domains:

```text
geometry
basic taxonomy
kinship relations
small legal definitions
measurement concepts
```

Avoid beginning with highly contextual concepts such as:

```text
good
large
fair
normal
healthy
intelligent
```

These should wait until the framework can represent context, measurement frames, and grounding explicitly.

---

# 17. Final architectural statement

Know should be built as:

```text
English and other natural languages
        ↓
lexical forms and contextual resolution
        ↓
canonical concept IDs
        ↓
RON-backed Rust ontology structures
        ↓
OWL-aligned description-logic semantics
        ↓
sound and complete reasoning for a documented fragment
        ↓
automated knowledge admission
        ↓
auditable cumulative knowledge
```

The project is not attempting to replace the logical work behind OWL.

It is building a modern Rust-native system around that work:

* cleaner serialization,
* canonical concept identity,
* explicit treatment of polysemy,
* LLM-first knowledge extraction,
* automated admission,
* transparent explanations,
* strong developer tooling,
* and an auditable semantic dataset.

The central rule should be:

> Natural-language words may remain ambiguous. Accepted Know concepts may not.
