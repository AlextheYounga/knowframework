//! Automated knowledge admission pipeline.
//!
//! An LLM (or other source) produces a `KnowledgeProposal`. The pipeline
//! validates it through a sequence of gates and emits an `AdmissionRecord`.
//!
//! Pipeline stages (§11 of the architecture plan):
//!   1. Structural validation — valid IDs, no missing references, no
//!      duplicate immutable IDs, no unsupported constructors.
//!   2. Lexical validation — surface words are not treated as canonical
//!      concepts; polysemous forms produce separate bindings.
//!   3. Ontological validation — definitions well-typed, grounding
//!      categories compatible, domain/range constraints satisfied.
//!   4. Logical validation — ontology remains consistent; new concepts
//!      are satisfiable; definitions don't collapse distinct concepts.
//!   5. Regression validation — previously accepted queries and invariants
//!      still hold.
//!
//! TODO: specify positive admission criteria. The current plan describes
//! rejection and deferral cases clearly but does not pin down what
//! "stable enough across independent runs" means quantitatively, or
//! how the pipeline resolves two structurally valid but mutually
//! incompatible proposals for the same concept.

use know_core::{Diagnostic, Provenance};
use know_lexicon::LexicalForm;
use know_ontology::{
    AxiomSource, ConceptRecordSource, EntityRecordSource, KnowledgeModule, RelationRecordSource,
};
use know_reasoner::Verdict;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Proposal
// ---------------------------------------------------------------------------

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
#[derive(Debug, Clone)]
pub struct KnowledgeProposal {
    pub proposal_id: String,
    pub proposed_concepts: Vec<ConceptRecordSource>,
    pub proposed_relations: Vec<RelationRecordSource>,
    pub proposed_entities: Vec<EntityRecordSource>,
    pub proposed_axioms: Vec<AxiomSource>,
    pub lexical_bindings: Vec<LexicalForm>,
    pub source_evidence: Vec<SourceEvidence>,
    pub generated_by: GeneratorIdentity,
}

// ---------------------------------------------------------------------------
// Decision and audit record
// ---------------------------------------------------------------------------

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

#[derive(Debug, Clone, Copy)]
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
    // Storing the proposition as a string until Proposition implements Display.
    pub proposition_description: String,
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
    /// ISO 8601 timestamp.
    pub timestamp: String,
    pub provenance: Provenance,
}

// ---------------------------------------------------------------------------
// Pipeline trait
// ---------------------------------------------------------------------------

pub trait AdmissionPipeline {
    /// Run the proposal through all validation stages and return a record.
    ///
    /// Does not mutate the knowledge module; callers apply accepted proposals
    /// themselves via whatever version-management layer they have.
    fn admit(&self, proposal: KnowledgeProposal, knowledge: &KnowledgeModule) -> AdmissionRecord;
}

/// Placeholder pipeline — rejects every proposal until implemented.
pub struct UnimplementedPipeline;

impl AdmissionPipeline for UnimplementedPipeline {
    fn admit(&self, proposal: KnowledgeProposal, _knowledge: &KnowledgeModule) -> AdmissionRecord {
        // TODO: implement the five-stage pipeline described in the module doc.
        AdmissionRecord {
            id: format!("audit:{}", proposal.proposal_id),
            proposal_id: proposal.proposal_id,
            decision: AdmissionDecision::Rejected(vec![Diagnostic::error(
                know_core::codes::UNSUPPORTED_FEATURE,
                "admission pipeline not yet implemented",
            )]),
            validation_results: vec![],
            changed_verdicts: vec![],
            generated_by: proposal.generated_by,
            timestamp: String::new(),
            provenance: Provenance::default(),
        }
    }
}
