//! A small trusted kernel for checking terms in Know's foundational calculus.
//!
//! The kernel checks explicit proof terms. Proof discovery, ontology reasoning,
//! and natural-language interpretation remain outside this crate.

mod checker;
mod term;
mod theory;

pub use checker::{CheckError, Checker};
pub use term::Term;
pub use theory::{CheckedTheorem, Declaration, DeclarationKind, Theory, TheoryError};
