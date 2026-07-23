//! Reasoning over a validated `KnowledgeModule`.
//!
//! The `Reasoner` trait defines the interface. Implementations live in
//! submodules; only one is currently provided (see below).
//!
//! Implementation strategy (staged per the architecture plan):
//!
//!   Stage 2 — Boolean concept fragment: AND / OR / NOT / subclass /
//!              equivalence / disjointness / class assertions.
//!              Back this with a complete Boolean decision procedure (SAT).
//!
//!   Stage 3 — Relational description logic: EXISTS / FOR_ALL restrictions,
//!              relation assertions. Requires a full ALC tableau or equivalent.
//!
//! Neither stage is implemented yet. The trait and verdict types are
//! established here so downstream crates can depend on stable interfaces.

use know_core::{AxiomId, ConceptId, Diagnostic, EntityId, RelationId};
use know_ontology::{ConceptExpr, KnowledgeModule};

// ---------------------------------------------------------------------------
// Propositions — what the reasoner can be asked about
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Proposition {
    /// Is `entity` an instance of `class`?
    ClassMembership { entity: EntityId, class: ConceptExpr },
    /// Is every instance of `child` also an instance of `parent`?
    SubclassOf { child: ConceptExpr, parent: ConceptExpr },
    /// Do `left` and `right` have exactly the same instances?
    Equivalent { left: ConceptExpr, right: ConceptExpr },
    /// Can no individual simultaneously belong to both?
    Disjoint { left: ConceptExpr, right: ConceptExpr },
    /// Does at least one possible interpretation satisfy `class`?
    Satisfiable { class: ConceptExpr },
    /// Does `subject relation object` hold?
    RelationHolds { subject: EntityId, relation: RelationId, object: EntityId },
    /// Is the entire knowledge module free of contradictions?
    Consistent,
}

// ---------------------------------------------------------------------------
// Verdicts
// ---------------------------------------------------------------------------

/// The logical conclusion the reasoner reached about a `Proposition`.
#[derive(Debug, Clone)]
pub enum Verdict {
    /// True in every model of the accepted ontology.
    Entailed(Explanation),
    /// False in every model (its negation is entailed).
    Contradicted(Explanation),
    /// Some models satisfy it, some do not — genuinely open under OWA.
    Unknown(UnknownExplanation),
    /// The proposition cannot be formed: a word or expression has multiple
    /// unresolved formalizations. This is a pre-reasoning lexical failure.
    Ambiguous(AmbiguityReport),
    /// The proposition applies concepts or relations to incompatible categories.
    IllTyped(Vec<Diagnostic>),
    /// No model satisfies all accepted axioms simultaneously.
    Inconsistent(InconsistencyReport),
}

/// Distinguishes how a reasoning session ended from what it concluded.
///
/// `Unknown` in `Verdict` means the ontology is genuinely open on the
/// question. `Unsupported` or `ResourceLimit` here mean the reasoner
/// could not complete the query — the ontology's openness is irrelevant.
#[derive(Debug, Clone)]
pub enum ReasoningOutcome<T> {
    Complete(T),
    /// The query used a construct outside the reasoner's supported fragment.
    Unsupported(UnsupportedFeature),
    /// Reasoning was halted before completion due to resource limits.
    ResourceLimit(ResourceLimit),
    InternalError(String),
}

// ---------------------------------------------------------------------------
// Explanation and supporting types
// ---------------------------------------------------------------------------

/// An explanation attached to `Entailed` or `Contradicted`.
///
/// `steps` gives the inference chain. `supporting_axioms` lists every axiom
/// from the module that was used, enabling dependency tracking.
#[derive(Debug, Clone)]
pub struct Explanation {
    pub conclusion: Proposition,
    pub supporting_axioms: Vec<AxiomId>,
    pub steps: Vec<InferenceStep>,
}

#[derive(Debug, Clone)]
pub struct InferenceStep {
    pub rule: InferenceRule,
    pub premises: Vec<Proposition>,
    pub conclusion: Proposition,
}

/// The inference rules the reasoner knows about.
///
/// Extended as new reasoning stages are added.
/// TODO: expand with ALC tableau rules when Stage 3 is implemented.
#[derive(Debug, Clone)]
pub enum InferenceRule {
    SubclassPropagation,
    DefinitionExpansion,
    EquivalenceUnfolding,
    DisjointnessContradiction,
    ClassAssertionPropagation,
    ComplementContradiction,
}

/// Explanation for an `Unknown` verdict: what the ontology is missing.
#[derive(Debug, Clone)]
pub struct UnknownExplanation {
    pub proposition: Proposition,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AmbiguityReport {
    pub candidates: Vec<ConceptId>,
}

#[derive(Debug, Clone)]
pub struct InconsistencyReport {
    pub conflicting_axioms: Vec<AxiomId>,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UnsupportedFeature {
    pub feature: String,
    pub advice: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceLimit {
    pub kind: ResourceLimitKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ResourceLimitKind {
    Time,
    Memory,
    Iterations,
}

// ---------------------------------------------------------------------------
// Classification result (batch subclass queries)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Direct named superclasses of the queried concept.
    pub direct_superclasses: Vec<ConceptId>,
    /// Direct named subclasses.
    pub direct_subclasses: Vec<ConceptId>,
    /// Concepts whose extension is provably identical to this one.
    pub equivalent_classes: Vec<ConceptId>,
}

// ---------------------------------------------------------------------------
// Reasoner trait
// ---------------------------------------------------------------------------

pub trait Reasoner {
    /// Evaluate a single proposition against the loaded knowledge module.
    fn query(&self, proposition: &Proposition) -> ReasoningOutcome<Verdict>;

    /// Compute the position of a concept in the subclass hierarchy.
    fn classify(&self, concept: &ConceptExpr) -> ReasoningOutcome<ClassificationResult>;

    /// Check whether the module is globally consistent.
    fn is_consistent(&self) -> ReasoningOutcome<bool>;

    /// Check whether a concept expression has at least one possible instance.
    fn is_satisfiable(&self, concept: &ConceptExpr) -> ReasoningOutcome<bool>;
}

// ---------------------------------------------------------------------------
// Placeholder implementation
//
// Returns `Unsupported` for every query until Stage 2 is built.
// Exists so `know-cli` and `know-admission` can compile and run.
// ---------------------------------------------------------------------------

pub struct UnimplementedReasoner;

impl UnimplementedReasoner {
    pub fn new(_module: &KnowledgeModule) -> Self {
        Self
    }
}

impl Reasoner for UnimplementedReasoner {
    fn query(&self, _proposition: &Proposition) -> ReasoningOutcome<Verdict> {
        // TODO: implement Stage 2 Boolean SAT backend, then Stage 3 ALC tableau.
        ReasoningOutcome::Unsupported(UnsupportedFeature {
            feature: "reasoning".to_string(),
            advice: Some("Boolean SAT backend not yet implemented (Stage 2).".to_string()),
        })
    }

    fn classify(&self, _concept: &ConceptExpr) -> ReasoningOutcome<ClassificationResult> {
        ReasoningOutcome::Unsupported(UnsupportedFeature {
            feature: "classification".to_string(),
            advice: None,
        })
    }

    fn is_consistent(&self) -> ReasoningOutcome<bool> {
        ReasoningOutcome::Unsupported(UnsupportedFeature {
            feature: "consistency".to_string(),
            advice: None,
        })
    }

    fn is_satisfiable(&self, _concept: &ConceptExpr) -> ReasoningOutcome<bool> {
        ReasoningOutcome::Unsupported(UnsupportedFeature {
            feature: "satisfiability".to_string(),
            advice: None,
        })
    }
}
