use know_core::{Diagnostic, Provenance};
use know_ontology::{AxiomSource, ConceptRecordSource, EntityRecordSource, RelationRecordSource};
use know_lexicon::LexicalForm;
use know_reasoner::{Proposition, Verdict};
use serde::{Deserialize, Serialize};

/// Source evidence that accompanied the LLM's extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEvidence {
    pub kind: EvidenceKind,
    pub text: String,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceKind {
    Text,
    WebPage,
    Document,
    HumanAnnotation,
    LlmExtraction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorIdentity {
    pub kind: GeneratorKind,
    pub model_id: Option<String>,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeneratorKind {
    Llm,
    Human,
    Import,
    Automated,
}

/// A bundle of additions proposed by an LLM or other source.
///
/// Proposals use source-layer types so they can arrive as raw RON before
/// any resolution occurs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeProposal {
    pub proposal_id: String,
    #[serde(default)]
    pub proposed_concepts: Vec<ConceptRecordSource>,
    #[serde(default)]
    pub proposed_relations: Vec<RelationRecordSource>,
    #[serde(default)]
    pub proposed_entities: Vec<EntityRecordSource>,
    #[serde(default)]
    pub proposed_axioms: Vec<AxiomSource>,
    #[serde(default)]
    pub lexical_bindings: Vec<LexicalForm>,
    #[serde(default)]
    pub source_evidence: Vec<SourceEvidence>,
    pub generated_by: GeneratorIdentity,
}

impl KnowledgeProposal {
    pub fn from_ron(input: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(input)
    }
}

#[derive(Debug, Clone)]
pub enum AdmissionDecision {
    Accepted,
    AcceptedWithWarnings(Vec<Diagnostic>),
    Rejected(Vec<Diagnostic>),
    /// Lexical interpretation of at least one term is unresolved.
    DeferredForAmbiguity(Vec<Diagnostic>),
    /// At least one concept lacks the grounding needed to validate it.
    DeferredForGrounding(Vec<Diagnostic>),
    /// The proposal is logically incompatible with already-accepted knowledge.
    ConflictsWithExistingKnowledge(Vec<Diagnostic>),
}

/// The result of running a single pipeline stage.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub stage: ValidationStage,
    pub passed: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStage {
    Structural,
    Lexical,
    Ontological,
    Logical,
    Regression,
}

/// A before/after change in verdict caused by the proposed knowledge.
#[derive(Debug, Clone)]
pub struct VerdictDiff {
    pub proposition: String,
    pub before: Option<Verdict>,
    pub after: Option<Verdict>,
}

/// Full audit record for one admission attempt.
#[derive(Debug, Clone)]
pub struct AdmissionRecord {
    pub id: String,
    pub proposal_id: String,
    pub decision: AdmissionDecision,
    pub validation_results: Vec<ValidationResult>,
    pub changed_verdicts: Vec<VerdictDiff>,
    pub generated_by: GeneratorIdentity,
    /// ISO 8601 timestamp (UTC).
    pub timestamp: String,
    pub provenance: Provenance,
}

/// A previously accepted verdict that must survive new knowledge.
///
/// Every semantic bug becomes a permanent check here (plan §14).
#[derive(Debug, Clone)]
pub struct RegressionCheck {
    pub description: String,
    pub proposition: Proposition,
    pub expected: ExpectedVerdict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedVerdict {
    Entailed,
    Contradicted,
    Unknown,
}

impl ExpectedVerdict {
    pub(crate) fn matches(self, verdict: &Verdict) -> bool {
        matches!(
            (self, verdict),
            (ExpectedVerdict::Entailed, Verdict::Entailed(_))
                | (ExpectedVerdict::Contradicted, Verdict::Contradicted(_))
                | (ExpectedVerdict::Unknown, Verdict::Unknown(_))
        )
    }
}
