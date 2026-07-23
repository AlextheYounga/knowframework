use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Stable opaque identifiers
//
// Each ID space is a distinct newtype so the compiler prevents mixing them.
// IDs are strings rather than integers: they carry human-readable namespaced
// paths (e.g. "geometry::square") and remain stable across serialization.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConceptId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelationId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AxiomId(pub String);

// BCP 47 language tag, e.g. "en" or "en-US".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LanguageId(pub String);

impl LanguageId {
    pub fn english() -> Self {
        Self("en".to_string())
    }
}

// ---------------------------------------------------------------------------
// Provenance
//
// Every accepted piece of knowledge must be traceable to how it arrived.
// The timestamp is an ISO 8601 string to avoid a chrono dependency here.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub source: ProvenanceSource,
    pub timestamp: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProvenanceSource {
    LlmGenerated { model: String, run_id: String },
    HumanAuthored { author: Option<String> },
    Imported { source_iri: String },
    Derived { from: Vec<String> },
    Unknown,
}

impl Default for Provenance {
    fn default() -> Self {
        Self { source: ProvenanceSource::Unknown, timestamp: None, notes: None }
    }
}

// ---------------------------------------------------------------------------
// Semantic versioning for knowledge modules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ---------------------------------------------------------------------------
// Diagnostics
//
// Structured errors and warnings produced by every validation pass.
// Uses structural data paths rather than source spans because RON does not
// naturally expose span information for every deserialized field.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub related: Vec<RelatedInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: Option<PathBuf>,
    /// Structural data path into the deserialized document,
    /// e.g. "concepts[3].definition.And[1]".
    pub data_path: Option<String>,
    // TODO: RON span (line/col) if the ron crate's SpannedValue API can be
    // threaded through deserialization without requiring a full custom parser.
}

#[derive(Debug, Clone)]
pub struct RelatedInfo {
    pub message: String,
    pub location: Option<SourceLocation>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, code, message: message.into(), location: None, related: vec![] }
    }

    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Warning, code, message: message.into(), location: None, related: vec![] }
    }

    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.location.get_or_insert(SourceLocation { file: None, data_path: None }).file = Some(path);
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.location.get_or_insert(SourceLocation { file: None, data_path: None }).data_path = Some(path.into());
        self
    }
}

// Stable diagnostic codes. Kept here so all crates reference one source.
pub mod codes {
    pub const MALFORMED_RON: &str = "K001";
    pub const UNSUPPORTED_SCHEMA_VERSION: &str = "K002";
    pub const DUPLICATE_ID: &str = "K003";
    pub const UNRESOLVED_CONCEPT: &str = "K004";
    pub const UNRESOLVED_RELATION: &str = "K005";
    pub const UNRESOLVED_ENTITY: &str = "K006";
    pub const DUPLICATE_DEFINITION: &str = "K007";
    pub const CIRCULAR_DEFINITION: &str = "K008";
    pub const UNSATISFIABLE_CONCEPT: &str = "K009";
    pub const INCONSISTENT_ENTITY: &str = "K010";
    pub const INVALID_SENSE_BINDING: &str = "K011";
    pub const AMBIGUOUS_SENSE: &str = "K012";
    pub const ADMISSION_REGRESSION: &str = "K013";
    pub const UNSUPPORTED_FEATURE: &str = "K014";
}
