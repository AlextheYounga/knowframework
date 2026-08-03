# Contributing to the Knowledge Corpus

This directory tells coding agents how to change the knowledge corpus under
`knowledge/`. Read these documents before editing a `.know` file:

1. [CORPUS_FORMAT.md](CORPUS_FORMAT.md) describes the supported file schemas.
2. [MODELING_GUIDE.md](MODELING_GUIDE.md) explains how to model knowledge
   without asserting more than the evidence supports.
3. [VALIDATION.md](VALIDATION.md) gives the required verification workflow.

The existing geometry package in `knowledge/geometry/` is the canonical
working example.

## Non-Negotiable Rules

- Treat generated knowledge as a proposal, not as accepted fact.
- Never invent evidence, provenance, identifiers, definitions, or relations.
- Use stable, namespaced IDs such as `geometry::square`.
- Keep concepts separate from words. Concepts belong in an ontology module;
  words and senses belong in a lexicon.
- Do not turn an ambiguous word into an `Or` expression. Preserve each sense
  as a separate concept binding.
- Use `Unknown` as the expected result when neither a claim nor its negation
  follows. Missing information is not evidence of falsity.
- Do not approximate unsupported relational reasoning. `Exists`, `ForAll`,
  and relation assertions compile, but the current Boolean reasoner does not
  reason over them.
- Do not reference concepts, relations, or entities from another ontology
  module. Cross-module ontology imports are not implemented.
- Run the checks in [VALIDATION.md](VALIDATION.md) before reporting completion.

## Choose the Right Contribution

Edit an ontology module when adding or correcting canonical concepts,
relations, entities, or logical axioms.

Edit a lexicon when adding a word form, language, part of speech, usage
example, or contextual sense binding. A lexical binding does not create its
target concept.

Create a proposal when knowledge is newly extracted, generated, uncertain,
or should be reviewed by the admission pipeline before acceptance. Prefer
this route for agent-generated additions.

## Recommended Agent Workflow

1. Read the target package and search for existing IDs and equivalent ideas.
2. State the intended claims in plain language.
3. Separate direct evidence from logical consequences.
4. Select the smallest appropriate contribution type.
5. Add provenance or source evidence when it is known.
6. Validate syntax and structural references.
7. Query the important intended and unintended consequences.
8. Add or update runnable tests when the contribution establishes behavior
   that must remain stable.
9. Report changed files, validation commands, and any remaining uncertainty.

## Current Boundaries

- Schema version `1` is the only supported ontology schema.
- `.know` files use RON syntax; there is no custom Know parser.
- Ontology references resolve only within the loaded module.
- Boolean concept reasoning is implemented for `Named`, `And`, `Or`, and
  `Not` expressions.
- OWL import and complete OWL export are not available.
- `Know.toml` records package intent but is not currently loaded by the CLI.
- Admission grounding policy and persistent acceptance are not complete.

These limits must be documented rather than worked around with guessed or
weakened semantics.
