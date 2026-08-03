# Corpus Validation

Every corpus change must be checked at three levels: structure, intended
semantics, and regressions.

The commands below assume the repository root as the working directory.

## 1. Structural Validation

Check every changed ontology module:

```bash
cargo run --quiet -p know-cli -- check knowledge/geometry/concepts.know
```

A successful check prints:

```text
ok
```

This verifies RON parsing, schema version, duplicate IDs, status/definition
agreement, local references, and definition cycles. It does not prove that the
claims are factually correct or that the module is logically consistent.

To inspect canonical serialization without overwriting the source:

```bash
cargo run --quiet -p know-cli -- normalize knowledge/geometry/concepts.know
```

Review normalization output manually. Do not replace a file wholesale unless
normalization is an explicit goal.

Lexicons do not currently have a standalone `check` command. Exercise each
new or changed form through `resolve`:

```bash
cargo run --quiet -p know-cli -- \
  resolve knowledge/geometry/lexicon.know square
```

For polysemous forms, test no-context ambiguity and contextual resolution:

```bash
cargo run --quiet -p know-cli -- \
  resolve knowledge/geometry/lexicon.know diamond

cargo run --quiet -p know-cli -- \
  resolve knowledge/geometry/lexicon.know diamond \
  --context geometry::polygon
```

## 2. Semantic Checks

Query each important consequence introduced by the change.

Subclass:

```bash
cargo run --quiet -p know-cli -- \
  reason knowledge/geometry/concepts.know \
  subclass geometry::square geometry::polygon
```

Satisfiability of a compound expression:

```bash
cargo run --quiet -p know-cli -- \
  reason knowledge/geometry/concepts.know \
  satisfiable 'And([Named("geometry::square"),Named("geometry::circle")])'
```

Detailed explanation:

```bash
cargo run --quiet -p know-cli -- \
  explain knowledge/geometry/concepts.know \
  subclass geometry::square geometry::polygon
```

Classification:

```bash
cargo run --quiet -p know-cli -- \
  classify knowledge/geometry/concepts.know geometry::square
```

Check both intended and dangerous unintended consequences. At minimum ask:

- Does the new child classify under its intended parent?
- Did a definition accidentally make two concepts equivalent?
- Did new disjointness make a valid concept unsatisfiable?
- Did entity assertions make the module inconsistent?
- Do deliberately unstated claims remain `Unknown`?

## 3. Proposal Admission

Run agent-generated or extracted knowledge through admission:

```bash
cargo run --quiet -p know-cli -- \
  admit knowledge/geometry/concepts.know \
  knowledge/geometry/proposals/pentagon.know
```

Read every stage and the final `decision:` line. The command currently exits
successfully after printing an admission record even when the decision is a
rejection or conflict, so process exit status alone is insufficient.

Admission currently checks structural validity, lexical target IDs, Boolean
logical consistency, concept collapse, and configured in-memory regression
checks. Grounding policy is not implemented, CLI-loaded regression manifests
are not available, and an accepted CLI decision is not persisted into the
base module.

## 4. Automated Tests

Run tests for crates whose behavior or fixtures changed. For corpus semantics,
the current acceptance suite is:

```bash
cargo test -p know-reasoner --test geometry
```

For admission changes:

```bash
cargo test -p know-admission --test admission_tests
```

For compiler or schema changes:

```bash
cargo test -p know-ontology --test compile_tests
```

Run all implementation tests when the change spans packages:

```bash
cargo test --workspace --exclude cleancode
```

The clean-code package is a separate repository-policy suite. At the time this
guide was written, some of its broad text/AST checks report existing findings;
do not describe those findings as corpus semantic failures.

## 5. Regression Tests

Add a runnable test when a contribution establishes a semantic promise that
must survive future changes. Shared programmatic corpus fixtures belong in
`tests/fixtures`; assertions belong in the relevant crate's test suite.

Prefer observable claims such as:

- expected verdict for a proposition,
- expected direct classification,
- expected ambiguity or contextual resolution,
- expected proposal admission decision.

Do not test private implementation details or duplicate every transitive
consequence.

## Completion Report

An agent's final report should state:

- corpus files changed,
- claims added, removed, or corrected,
- evidence and provenance used,
- commands run and their outcomes,
- expected verdicts checked,
- unresolved ambiguity, unsupported semantics, or missing evidence.

Never report a corpus contribution as validated if only formatting or parsing
was checked.
