# ADR-0001: Use Rust to implement one canonical knowledge language

## Status

Accepted for the initial framework.

## Context

The project needs a fast, deterministic implementation with a small trusted core, clear
ownership of immutable syntax trees, explicit errors, and strong support for testing and
distribution. The knowledge language must retain semantics independent of its
implementation language.

## Decision

Implement the parser, AST, core checker, revision gate, and CLI in a Rust workspace.
Define a separate `.know` source language rather than encoding knowledge directly as
Rust types or macros.

## Consequences

### Positive

- Memory safety without a garbage collector.
- Exhaustive enums suit syntax, type, proof, and diagnostic trees.
- The core can avoid runtime and network dependencies.
- One canonical language remains portable to other implementations.

### Negative

- The team must implement language infrastructure rather than inheriting all of a proof
  assistant's elaborator.
- Rust's type system is not itself the knowledge calculus; two type systems must be
  designed and maintained carefully.

### Neutral

- External theorem systems remain useful as validators and proof-search engines.

## Alternatives Considered

- Implement directly in Lean: stronger proof foundations, but a less conventional
  systems and distribution environment for the primary toolchain.
- Encode knowledge as Rust APIs or macros: quick to prototype, but would couple the
  knowledge language's semantics to Rust implementation details.
- Begin in Python: faster initial experimentation, but a weaker fit for the eventual
  small trusted runtime and deterministic distribution goal.
