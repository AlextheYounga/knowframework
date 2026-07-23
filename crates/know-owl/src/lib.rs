//! OWL interoperability (Phase 5 — not yet implemented).
//!
//! Planned responsibilities:
//!   - Export the supported Know fragment to OWL 2 Functional Syntax.
//!   - Import the corresponding supported OWL subset.
//!   - Maintain stable mappings between Know IDs and OWL IRIs.
//!   - Drive differential testing against a mature OWL reasoner
//!     (e.g. HermiT or Openllet) to validate Know's reasoning results.
//!
//! The correct compatibility claim will be:
//!   "Know implements OWL-aligned semantics for a documented subset."
//!   Not full OWL 2 DL compatibility.
//!
//! TODO: design the IRI mapping scheme. Know IDs like "geometry::square"
//! must map to stable IRIs without collisions and without requiring the
//! IRI to change when a concept is renamed (human labels may change;
//! identity must not).
//!
//! TODO: decide which OWL 2 profile (EL / QL / RL / DL) is the right
//! comparison target for Know V1's Boolean+ALC fragment.

use know_ontology::KnowledgeModule;

/// Export a Know module to OWL 2 Functional Syntax.
///
/// Not yet implemented. Returns an error string for now so callers compile.
pub fn export_owl_functional(module: &KnowledgeModule) -> Result<String, String> {
    let _ = module;
    Err("OWL export not yet implemented (Phase 5)".to_string())
}

/// Import an OWL 2 Functional Syntax ontology into a Know source module.
///
/// Not yet implemented.
pub fn import_owl_functional(_owl_source: &str) -> Result<know_ontology::KnowledgeModuleSource, String> {
    Err("OWL import not yet implemented (Phase 5)".to_string())
}
