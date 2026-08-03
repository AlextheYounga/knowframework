# Corpus Format

Knowledge packages live under `knowledge/<domain>/`. The geometry package is
the reference layout:

```text
knowledge/geometry/
├── Know.toml
├── concepts.know
├── lexicon.know
└── proposals/
    └── pentagon.know
```

All `.know` files are Rusty Object Notation (RON). Type and enum names are
case-sensitive.

## Ontology Modules

An ontology file deserializes as `KnowledgeModuleSource`:

```ron
KnowledgeModuleSource(
    id: "astronomy",
    schema_version: 1,
    concepts: [
        ConceptRecordSource(
            id: "astronomy::celestial_body",
            label: "celestial body",
            alternate_labels: [],
            definition: None,
            grounding: Some(NaturalKind),
            status: Primitive,
            provenance: Some((
                source: HumanAuthored(author: Some("contributor")),
                timestamp: None,
                notes: Some("Foundational concept for this module"),
            )),
        ),
        ConceptRecordSource(
            id: "astronomy::planet",
            label: "planet",
            alternate_labels: [],
            definition: None,
            grounding: Some(NaturalKind),
            status: Declared,
            provenance: None,
        ),
    ],
    relations: [],
    entities: [],
    axioms: [
        SubclassOf(
            child: Named("astronomy::planet"),
            parent: Named("astronomy::celestial_body"),
        ),
    ],
)
```

### IDs

- Use `<module>::<local_name>`.
- Use lowercase `snake_case` for the local name.
- IDs are stable identities, not display text. Do not rename an ID only to
  improve capitalization or wording.
- Every ontology reference must resolve in the same file for now.
- IDs must be unique across concepts, unique across relations, and unique
  across entities. Avoid reusing one ID across categories even though the
  current compiler checks each category independently.

### Concept Status

`Primitive`
: Foundational for the current model. It has no complete definition.

`Declared`
: Classified by necessary conditions or subclass axioms, but has no complete
  necessary-and-sufficient definition.

`Defined`
: Has a complete `definition`. The definition is interpreted as necessary and
  sufficient.

`Deprecated`
: Retained for version history but no longer current.

`Defined` requires `definition: Some(...)`. `Primitive` and `Declared` require
`definition: None`. Definitions must not form direct or indirect cycles.

### Grounding

Available grounding values are:

```text
NaturalKind
StructuralDefinition
FunctionalKind
SocialKind
LegalKind
Role
Process
Event
MeasurementClass
EngineeredConcept
Primitive
```

Grounding is currently metadata. Compatibility and inheritance rules are not
implemented, so select a value only when its meaning is supported. Use
`None` rather than guessing; compilation currently maps absent grounding to
`Primitive` in the internal representation.

### Concept Expressions

```ron
Named("domain::concept")
And([Named("domain::a"), Named("domain::b")])
Or([Named("domain::a"), Named("domain::b")])
Not(Named("domain::concept"))
Exists(relation: "domain::relation", filler: Named("domain::concept"))
ForAll(relation: "domain::relation", filler: Named("domain::concept"))
```

`Exists` and `ForAll` are valid source syntax, but current reasoning over them
is unsupported. Do not use them when a contribution requires an executable
proof today.

### Relations and Entities

```ron
RelationRecordSource(
    id: "domain::has_part",
    label: "has part",
    domain: Some(Named("domain::whole")),
    range: Some(Named("domain::part")),
    provenance: None,
)

EntityRecordSource(
    id: "domain::example_one",
    label: "example one",
    provenance: None,
)
```

Relation domain and range are metadata in the current reasoning stage. Entity
membership can be queried when expressed with class assertions.

### Axioms

```ron
SubclassOf(child: Named("domain::child"), parent: Named("domain::parent"))
EquivalentClasses(classes: [Named("domain::a"), Named("domain::b")])
DisjointClasses(classes: [Named("domain::a"), Named("domain::b")])
ClassAssertion(entity: "domain::entity", class: Named("domain::concept"))
NegativeClassAssertion(entity: "domain::entity", class: Named("domain::concept"))
RelationAssertion(
    subject: "domain::subject",
    relation: "domain::relation",
    object: "domain::object",
)
NegativeRelationAssertion(
    subject: "domain::subject",
    relation: "domain::relation",
    object: "domain::object",
)
```

Boolean reasoning supports class-level axioms and positive or negative class
assertions. Relational assertions are structurally validated but not reasoned
over yet.

## Lexicon Modules

A lexicon maps surface forms to existing canonical concept IDs:

```ron
LexicalModule(
    language: "en",
    forms: [
        LexicalForm(
            text: "planet",
            language: "en",
            part_of_speech: Some(Noun),
            bindings: [
                LexicalBinding(
                    concept: "astronomy::planet",
                    context_hints: ["astronomy::celestial_body"],
                    usage_examples: ["the planet follows an orbit"],
                    provenance: (
                        source: HumanAuthored(author: Some("contributor")),
                        timestamp: None,
                        notes: None,
                    ),
                ),
            ],
        ),
    ],
)
```

Supported parts of speech are `Noun`, `Verb`, `Adjective`, `Adverb`, and
`Preposition`. Language values should be BCP 47 tags such as `en` or `en-US`.

A form may have multiple bindings for genuine polysemy. Give each binding
specific context hints and usage examples. Do not create a synthetic union
concept for the word.

The standalone `resolve` command does not verify that target concept IDs exist
in an ontology. Admission does validate lexical bindings against the combined
base module and proposal.

## Knowledge Proposals

Agents should normally place generated additions in a proposal:

```ron
KnowledgeProposal(
    proposal_id: "astronomy-planet-001",
    proposed_concepts: [
        ConceptRecordSource(
            id: "astronomy::planet",
            label: "planet",
            alternate_labels: [],
            definition: None,
            grounding: Some(NaturalKind),
            status: Declared,
            provenance: None,
        ),
    ],
    proposed_axioms: [
        SubclassOf(
            child: Named("astronomy::planet"),
            parent: Named("astronomy::celestial_body"),
        ),
    ],
    source_evidence: [
        SourceEvidence(
            kind: Document,
            text: "Exact supporting excerpt",
            reference: Some("source identifier or URL"),
        ),
    ],
    generated_by: (
        kind: Llm,
        model_id: Some("model-name"),
        run_id: "run-identifier",
    ),
)
```

Omitted proposal collections default to empty. Available evidence kinds are
`Text`, `WebPage`, `Document`, `HumanAnnotation`, and `LlmExtraction`.
Generator kinds are `Llm`, `Human`, `Import`, and `Automated`.

Never fabricate an excerpt, reference, model name, run ID, author, or
timestamp. If evidence is unavailable, leave `source_evidence` empty and make
the uncertainty explicit in the contribution report.

## Package Manifest

`Know.toml` currently documents package metadata and intended validation
policy. The CLI does not load or enforce it yet. Keep it accurate, but do not
claim that a successful module check proves its policy fields were applied.
