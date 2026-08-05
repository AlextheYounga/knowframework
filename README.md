# Know

Know is a Rust framework for building, validating, and reasoning over discrete semantic knowledge. It keeps canonical concepts separate from the words used to describe them, gives accepted claims explicit logical meaning, and records enough provenance and reasoning detail for results to be inspected.

> [!IMPORTANT]
> Know is an early-stage research prototype. The repository contains a working ontology compiler, lexical resolver, reasoners, OWL exporter, and proposal admission pipeline. It does not yet implement the complete self-editing knowledge language described in [`docs/idea.md`](docs/idea.md).

## Why Know?

Knowledge systems often blur together language, meaning, and inference. Know treats them as distinct layers:

- **Words are not concepts.** A word such as "diamond" can refer to several stable concept IDs. Context resolves the intended sense before reasoning begins.
- **Unknown is not false.** Reasoning follows the open-world assumption: a claim is `Unknown` when neither it nor its negation follows.
- **Unsupported is not unknown.** A query outside a reasoner's supported fragment is reported explicitly instead of being approximated.
- **Generated knowledge is a proposal.** New claims pass structural, lexical, logical, and regression checks before they can modify a module.
- **Results are inspectable.** Entailed and contradicted verdicts can include supporting axioms and inference steps.

## Current Capabilities

- Parse RON-encoded `.know` ontology modules into a typed intermediate representation.
- Validate schema versions, IDs, references, concept definitions, and definition cycles.
- Answer subclass, equivalence, disjointness, satisfiability, membership, and consistency queries.
- Classify the direct superclasses, subclasses, and equivalent classes of a concept.
- Resolve lexical forms to canonical concepts while preserving genuine ambiguity.
- Evaluate knowledge proposals through a five-stage admission pipeline and apply clean proposals atomically.
- Export validated modules as OWL 2 Functional Syntax.
- Reason over the Boolean concept fragment with a native SAT-backed engine and over the current relational fragment through the rustdl adapter.

## Quick Start

Know requires a Rust toolchain with edition 2024 support (Rust 1.85 or newer).

```bash
git clone https://github.com/AlextheYounga/knowframework.git
cd knowframework
cargo build --release -p know-cli
```

The resulting binary is `target/release/know`. The repository's `compile.sh` script also builds the workspace and copies the binary to `bin/know`.

Validate the example geometry ontology:

```bash
cargo run --quiet -p know-cli -- check knowledge/geometry/concepts.know
```

```text
ok
```

Ask whether every square is a polygon:

```bash
cargo run --quiet -p know-cli -- \
  reason knowledge/geometry/concepts.know \
  subclass geometry::square geometry::polygon
```

```text
Entailed
  rustdl refuted a counterexample
```

Resolve an ambiguous word, then provide context:

```bash
cargo run --quiet -p know-cli -- \
  resolve knowledge/geometry/lexicon.know diamond

cargo run --quiet -p know-cli -- \
  resolve knowledge/geometry/lexicon.know diamond \
  --context geometry::polygon
```

Without context, `diamond` remains ambiguous between the geometry and mineralogy concepts. With the polygon context it resolves to `geometry::rhombus`.

## CLI

The implemented commands read RON-backed project files and print results to standard output.

| Command | Purpose |
| --- | --- |
| `know check <path>` | Parse and structurally validate an ontology module. |
| `know normalize <path>` | Validate and print the module in canonical RON form. |
| `know reason <module> <query...>` | Evaluate a proposition and print its verdict. |
| `know explain <module> <query...>` | Evaluate a proposition and print its full explanation. |
| `know classify <module> <concept>` | Print direct superclasses, subclasses, and equivalents. |
| `know resolve <lexicon> <text> [--context <ids>] [--domain <id>]` | Resolve a lexical form to canonical concepts. |
| `know admit <module> <proposal> [--regressions <path>] [--apply]` | Validate a proposal and optionally apply it atomically. |
| `know export-owl <module>` | Print the module as OWL 2 Functional Syntax. |

Run `cargo run --quiet -p know-cli -- --help` or `target/release/know <command> --help` for command-specific help.

### Query Forms

`reason` and `explain` accept these queries:

```text
consistent
satisfiable <expr>
subclass <expr> <expr>
equivalent <expr> <expr>
disjoint <expr> <expr>
member <entity-id> <expr>
```

An expression can be a bare concept ID or a RON concept expression:

```bash
cargo run --quiet -p know-cli -- \
  reason knowledge/geometry/concepts.know \
  satisfiable 'And([Named("geometry::square"),Named("geometry::circle")])'
```

Verdicts distinguish logical conclusions (`Entailed`, `Contradicted`, `Unknown`, `Ambiguous`, `IllTyped`, and `Inconsistent`) from incomplete execution outcomes such as `Unsupported`, `ResourceLimit`, and `InternalError`.

## Knowledge Packages

The canonical example is [`knowledge/geometry`](knowledge/geometry):

```text
knowledge/geometry/
|-- Know.toml
|-- concepts.know
|-- lexicon.know
|-- proposals/
|   `-- pentagon.know
`-- regressions.know
```

`.know` files use [RON](https://github.com/ron-rs/ron), not a custom parser. Ontology modules define concepts, relations, entities, and axioms. Lexicons map language-specific forms to those canonical IDs. Proposals hold candidate additions, while regression manifests preserve expected semantic behavior.

Stable IDs use the `<module>::<snake_case>` convention, for example `geometry::square`. Schema version `1` is currently the only supported ontology schema.

For the complete formats and modeling rules, see:

- [Corpus format](.agents/CORPUS_FORMAT.md)
- [Modeling guide](.agents/MODELING_GUIDE.md)
- [Corpus validation](.agents/VALIDATION.md)

## Proposal Admission

Run the example proposal against the geometry module and its semantic regressions:

```bash
cargo run --quiet -p know-cli -- admit \
  knowledge/geometry/concepts.know \
  knowledge/geometry/proposals/pentagon.know \
  --regressions knowledge/geometry/regressions.know
```

The pipeline reports structural, lexical, ontological, logical, and regression stages followed by a final admission decision. Add `--apply` only when you intend to replace the base module with a cleanly accepted merge. Application uses a same-directory temporary file and refuses to overwrite a base file that changed during validation.

An admission command without `--apply` currently exits successfully even when its printed decision is a rejection, conflict, or deferral. Automation must inspect the `decision:` line rather than relying only on the process exit status.

## Architecture

Know is a Cargo workspace split by responsibility:

| Crate | Responsibility |
| --- | --- |
| `know-core` | Typed IDs, provenance, versions, and diagnostics. |
| `know-ontology` | RON source schema, validation, compilation, and typed ontology IR. |
| `know-lexicon` | Lexical forms, concept bindings, context, and resolution. |
| `know-reasoner` | Reasoning contracts and the native SAT-backed Boolean reasoner. |
| `know-owl` | OWL 2 Functional Syntax export and the rustdl reasoning adapter. |
| `know-admission` | Proposal validation, regression checking, decisions, and merging. |
| `know-cli` | The `know` command-line interface. |
| `tests/fixtures` | Shared executable knowledge fixtures. |

Source modules first compile from string-based records into typed IDs and expressions. Reasoners consume only the validated IR; lexical resolution and proposal admission remain separate boundaries around that semantic core.

## Development

Run the implementation test suite:

```bash
cargo test --workspace --exclude cleancode
```

Useful focused suites include:

```bash
cargo test -p know-ontology --test compile_tests
cargo test -p know-reasoner --test geometry
cargo test -p know-admission --test admission_tests
```

Format changes with `cargo fmt --all`. The `cleancode` workspace package contains repository-policy checks and is intentionally separate from semantic validation.

Corpus changes must follow [the contribution guide](.agents/README.md), including checks for structure, intended consequences, dangerous unintended consequences, and regressions.

## Current Boundaries

- This is a foundation for the larger Know research direction, not a complete proof language or autonomous knowledge system.
- `import-owl` and `diff-reasoner` are CLI placeholders and return not-implemented errors.
- OWL import, cross-module ontology imports, immutable knowledge versioning, and LLM integration are not implemented.
- `Know.toml` records package intent but is not currently loaded or enforced by the CLI.
- The admission pipeline's ontological grounding policy remains a placeholder.
- The native Boolean reasoner does not approximate `Exists`, `ForAll`, or relation assertions; relational support is limited to the capabilities exposed by the rustdl adapter.

The broader design and its non-goals are documented in [A Self-Editing Language for Coherent Knowledge](docs/idea.md).

## License

No license has been declared for this project yet.
