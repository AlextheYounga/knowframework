//! Source-level schema: what is deserialized directly from `.know` RON files.
//!
//! All concept references are plain strings at this layer. Name resolution and
//! validation happen in `compile`, which produces the typed IR in `ir`.

use know_core::Provenance;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Concept expressions (source layer uses String for concept/relation names)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConceptExprSource {
    Named(String),
    And(Vec<ConceptExprSource>),
    Or(Vec<ConceptExprSource>),
    Not(Box<ConceptExprSource>),
    Exists { relation: String, filler: Box<ConceptExprSource> },
    ForAll { relation: String, filler: Box<ConceptExprSource> },
}

// ---------------------------------------------------------------------------
// Axioms (source layer)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AxiomSource {
    SubclassOf { child: ConceptExprSource, parent: ConceptExprSource },
    EquivalentClasses { classes: Vec<ConceptExprSource> },
    DisjointClasses { classes: Vec<ConceptExprSource> },
    ClassAssertion { entity: String, class: ConceptExprSource },
    RelationAssertion { subject: String, relation: String, object: String },
    NegativeClassAssertion { entity: String, class: ConceptExprSource },
    NegativeRelationAssertion { subject: String, relation: String, object: String },
}

// ---------------------------------------------------------------------------
// Concept status and grounding classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConceptStatus {
    /// Accepted as foundational; no definition given or required.
    Primitive,
    /// Has necessary and sufficient conditions via `definition`.
    Defined,
    /// Has necessary parent classifications but no complete definition.
    Declared,
    /// No longer current; retained for version history.
    Deprecated,
}

/// Ontological grounding classification.
///
/// Records what kind of thing a concept is. Used as metadata and for
/// type-compatibility checks during admission. Logical consequences of
/// grounding (e.g. cross-grounding disjointness) are not yet implemented.
///
/// TODO: specify which grounding combinations are automatically disjoint,
/// and whether grounding is inherited through subclass hierarchies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grounding {
    NaturalKind,
    StructuralDefinition,
    FunctionalKind,
    SocialKind,
    LegalKind,
    Role,
    Process,
    Event,
    MeasurementClass,
    EngineeredConcept,
    Primitive,
}

// ---------------------------------------------------------------------------
// Record source types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConceptRecordSource {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub alternate_labels: Vec<String>,
    pub definition: Option<ConceptExprSource>,
    pub grounding: Option<Grounding>,
    pub status: ConceptStatus,
    pub provenance: Option<Provenance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationRecordSource {
    pub id: String,
    pub label: String,
    /// The concept expression that must hold of the subject of this relation.
    pub domain: Option<ConceptExprSource>,
    /// The concept expression that must hold of the object of this relation.
    pub range: Option<ConceptExprSource>,
    pub provenance: Option<Provenance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityRecordSource {
    pub id: String,
    pub label: String,
    pub provenance: Option<Provenance>,
}

/// Top-level structure of a `.know` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeModuleSource {
    pub id: String,
    pub schema_version: u32,
    #[serde(default)]
    pub concepts: Vec<ConceptRecordSource>,
    #[serde(default)]
    pub relations: Vec<RelationRecordSource>,
    #[serde(default)]
    pub entities: Vec<EntityRecordSource>,
    #[serde(default)]
    pub axioms: Vec<AxiomSource>,
}

impl KnowledgeModuleSource {
    pub fn from_ron(input: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(input)
    }

    pub fn to_ron(&self) -> Result<String, ron::Error> {
        let pretty = ron::ser::PrettyConfig::default();
        ron::ser::to_string_pretty(self, pretty)
    }
}
