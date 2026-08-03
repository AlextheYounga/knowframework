# Modeling Guide

The corpus should contain precise claims that the reasoner can interpret, not
an accumulation of plausible-sounding text.

## Model Concepts, Not Words

A concept is a canonical meaning with a stable ID. A word is a language-level
form that may refer to one or more concepts.

For example, the word `diamond` can refer to a geometric shape or a mineral.
Those meanings require distinct IDs and lexical bindings. They must not be
represented as one concept or as a logical disjunction.

Labels improve readability but do not determine identity. Before creating a
concept, search IDs, labels, alternate labels, definitions, and lexicons for
an existing meaning.

## Assert Only What Is Supported

Separate these categories before editing:

- **Direct claim:** explicitly supported by the source or accepted domain
  definition.
- **Logical consequence:** follows from existing axioms and should usually be
  left for the reasoner rather than duplicated.
- **Interpretation:** plausible but not established; keep it out or submit it
  as a clearly evidenced proposal.
- **Lexical observation:** concerns terminology or usage and belongs in a
  lexicon, not the ontology.

Do not add `A subclass C` merely because examples of `A` are often `C`. A
subclass axiom says every possible member of `A` must be a member of `C`.

Do not add disjointness merely because two classes are usually different.
`DisjointClasses([A, B])` says no entity can belong to both classes.

## Necessary and Sufficient Conditions

Use `Declared` with `SubclassOf` for necessary classification:

```ron
ConceptRecordSource(
    id: "geometry::rectangle",
    label: "rectangle",
    alternate_labels: [],
    definition: None,
    grounding: Some(StructuralDefinition),
    status: Declared,
    provenance: None,
)

SubclassOf(
    child: Named("geometry::rectangle"),
    parent: Named("geometry::quadrilateral"),
)
```

Use `Defined` only when the expression gives complete necessary-and-sufficient
conditions:

```ron
ConceptRecordSource(
    id: "geometry::square",
    label: "square",
    alternate_labels: [],
    definition: Some(And([
        Named("geometry::rectangle"),
        Named("geometry::rhombus"),
    ])),
    grounding: Some(StructuralDefinition),
    status: Defined,
    provenance: None,
)
```

A definition makes the named concept equivalent to its expression. If only
one direction is justified, use a subclass axiom instead.

## Open-World Reasoning

Know uses the open-world assumption. If the corpus does not establish whether
a square is red, the result is `Unknown`, not `Contradicted`.

Use negative assertions only when there is explicit support for the negation:

```ron
NegativeClassAssertion(
    entity: "domain::sample",
    class: Named("domain::hazardous"),
)
```

Never add a negative claim merely because a positive claim is absent.

## Expected Verdicts

`Entailed`
: The queried proposition follows logically from the loaded knowledge.

`Contradicted`
: The negation of the queried proposition follows logically.

`Unknown`
: Neither the proposition nor its negation follows.

`Ambiguous`
: A lexical interpretation has multiple unresolved candidates.

`IllTyped`
: The query refers to invalid or unknown vocabulary.

`Inconsistent`
: Accepted assertions cannot all hold together.

`Unsupported`
: The query requires a reasoning feature that is not implemented.

`Unsupported` is an execution outcome rather than evidence for or against a
claim. Never rewrite unsupported knowledge into weaker Boolean claims simply
to obtain a verdict.

## Provenance and Evidence

Use provenance to record how a record entered the corpus. Use proposal source
evidence to record why a proposed claim is warranted.

- Quote source text exactly when supplying an excerpt.
- Prefer stable source identifiers or URLs.
- Record the actual generator kind, model, and run ID.
- Do not convert model confidence into logical truth.
- Do not cite a source that supports only a related or weaker claim.
- If sources conflict, preserve the conflict for review instead of selecting
  the convenient claim.

## Minimal Changes

- Reuse an existing canonical concept when meanings match.
- Add the weakest axiom that accurately captures the evidence.
- Avoid redundant transitive axioms. If `A` is a subclass of `B` and `B` of
  `C`, the reasoner can derive that `A` is a subclass of `C`.
- Do not reorder or normalize an entire file when changing one record.
- Keep unrelated domains in separate modules.
- Add lexical data only when it improves actual resolution behavior.

## Unsupported Areas

The source schema includes relations and quantified expressions in preparation
for later reasoning stages. Current contributors must not claim executable
relational semantics for them.

Cross-module ontology imports are also unavailable. A lexicon may name an
external concept as lexical data, but that does not make the concept available
to ontology compilation or Boolean reasoning.
