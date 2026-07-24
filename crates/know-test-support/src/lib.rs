//! Test fixtures and assertion helpers for Know crates.
//!
//! Import this crate as a `[dev-dependencies]` entry. It must not be used
//! in production code paths.

pub mod geometry;

/// Assert that a slice of diagnostics contains no errors.
pub fn assert_no_errors(diagnostics: &[know_core::Diagnostic]) {
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity == know_core::Severity::Error).collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}
