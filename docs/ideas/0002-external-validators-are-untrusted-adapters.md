# ADR-0002: Treat external validators as untrusted adapters

## Status

Accepted for the initial framework.

## Context

Lean, Prolog, SMT solvers, and other languages can test different classes of claims.
Allowing each system to define independent semantics would undermine the one-language
principle and enlarge the trusted computing base unpredictably.

## Decision

External systems receive explicit validation requests through a versioned process
protocol and return `VALID`, `INVALID`, or `UNKNOWN`. Their results are reports. They do
not become core proofs unless a future adapter supplies a certificate independently
checked by the Rust kernel.

## Consequences

### Positive

- Multiple tools can be compared or combined without fragmenting canonical semantics.
- Validator crashes and malformed output cannot mutate the accepted program.
- The trust level of every result remains explicit.

### Negative

- A `VALID` report alone is not proof-carrying.
- Translators and certificate checkers must be implemented for stronger integration.

### Neutral

- External processes require operational sandboxing and resource limits outside the
  core library.

## Alternatives Considered

- Trust every configured validator: simpler, but makes correctness depend on arbitrary
  tools and configuration.
- Avoid external validators entirely: keeps the core small, but discards existing proof
  and logic ecosystems that can accelerate research.
