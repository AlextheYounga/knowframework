//! Automated knowledge admission pipeline.
//!
//! An LLM (or other source) produces a `KnowledgeProposal`. The pipeline
//! validates it through a sequence of gates and emits an `AdmissionRecord`.
//!
//! Pipeline stages (§11 of the architecture plan):
//!
//! 1. Structural — the merged module compiles: valid IDs, no missing
//!    references, no duplicate immutable IDs, no definition cycles,
//!    status/definition consistency.
//! 2. Lexical — bindings reference real concepts; surface words are not
//!    silently promoted to canonical concepts.
//! 3. Ontological — grounding-category compatibility. TODO: unimplemented
//!    until the plan specifies which grounding combinations are compatible;
//!    the stage runs and passes vacuously so records show it was considered.
//! 4. Logical — the merged ontology stays consistent, proposed concepts are
//!    satisfiable, definitions don't silently collapse distinct concepts
//!    into equivalence.
//! 5. Regression — previously accepted verdicts still hold.
//!
//! Ordinary structurally and logically valid proposals are admitted
//! automatically; deferral variants (`DeferredForAmbiguity`,
//! `DeferredForGrounding`) are reserved for the lexical-ambiguity and
//! grounding checks once those are specified.
//!
//! TODO: the plan does not yet pin down positive admission criteria such as
//! "stable across independent generator runs", or arbitration between two
//! valid but mutually incompatible proposals for the same concept.

mod timestamp;
pub mod types;

pub use types::*;

use know_core::{ConceptId, Diagnostic, Severity, codes};
use know_lexicon::LexicalForm;
use know_ontology::{ConceptExpr, ConceptExprSource, KnowledgeModuleSource, compile};
use know_owl::RustdlReasoner;
use know_reasoner::{BooleanReasoner, Proposition, Reasoner, ReasoningOutcome, Verdict};

use self::timestamp::iso8601_now;

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

pub struct Pipeline {
    base: KnowledgeModuleSource,
    regression_checks: Vec<RegressionCheck>,
}

impl Pipeline {
    pub fn new(base: KnowledgeModuleSource) -> Self {
        Self { base, regression_checks: vec![] }
    }

    pub fn with_regression_checks(mut self, checks: Vec<RegressionCheck>) -> Self {
        self.regression_checks = checks;
        self
    }

    pub fn with_regression_manifest(mut self, manifest: RegressionManifest) -> Self {
        self.regression_checks = manifest.checks.into_iter().map(regression_check).collect();
        self
    }

    /// Return the deterministic source module that would result from applying
    /// `proposal`. This does not imply that the proposal was admitted.
    pub fn merged_source(&self, proposal: &KnowledgeProposal) -> KnowledgeModuleSource {
        merge(&self.base, proposal)
    }

    /// Return the merged source only when every required admission gate passed
    /// without warnings. Callers are responsible for durable storage.
    pub fn apply(&self, proposal: KnowledgeProposal) -> Result<KnowledgeModuleSource, AdmissionRecord> {
        let record = self.admit(proposal.clone());
        if matches!(record.decision, AdmissionDecision::Accepted) {
            Ok(self.merged_source(&proposal))
        } else {
            Err(record)
        }
    }

    /// Run the proposal through all validation stages and return a record.
    ///
    /// Does not mutate the base module; callers apply accepted proposals
    /// themselves via whatever version-management layer they have.
    pub fn admit(&self, proposal: KnowledgeProposal) -> AdmissionRecord {
        let mut stages: Vec<ValidationResult> = vec![];
        let mut changed_verdicts: Vec<VerdictDiff> = vec![];
        let mut conflict = false;

        // --- 1. Structural: does the merged module compile? -----------------
        let merged_source = merge(&self.base, &proposal);
        let compiled = compile::compile(merged_source.clone());
        let structural_diags = match &compiled {
            Ok(_) => vec![],
            Err(e) => match e {
                compile::CompileError::UnsupportedSchemaVersion { found, expected } => {
                    vec![Diagnostic::error(
                        codes::UNSUPPORTED_SCHEMA_VERSION,
                        format!("schema version {found}, expected {expected}"),
                    )]
                }
                compile::CompileError::ValidationErrors { diagnostics, .. } => diagnostics.clone(),
            },
        };
        stages.push(stage(ValidationStage::Structural, structural_diags));

        // --- 2. Lexical -----------------------------------------------------
        stages.push(stage(ValidationStage::Lexical, check_lexical(&proposal.lexical_bindings, &merged_source)));

        // --- 3. Ontological -------------------------------------------------
        // Grounding compatibility rules are not yet specified (see module
        // doc); the stage exists so audit records show it ran.
        stages.push(stage(ValidationStage::Ontological, vec![]));

        // --- 4 & 5. Logical and regression (need a compiled module) --------
        if let Ok(module) = &compiled {
            match RustdlReasoner::new(module) {
                Ok(reasoner) => {
                    let (logical_diags, is_conflict) = check_logical(&reasoner, module, &proposal, &self.base);
                    conflict = is_conflict;
                    stages.push(stage(ValidationStage::Logical, logical_diags));

                    if !conflict {
                        let (regression_diags, diffs) = self.check_regression(&reasoner);
                        changed_verdicts = diffs;
                        stages.push(stage(ValidationStage::Regression, regression_diags));
                    }
                }
                Err(error) => stages.push(stage(
                    ValidationStage::Logical,
                    vec![Diagnostic::error(
                        codes::UNSUPPORTED_FEATURE,
                        format!("could not initialize rustdl: {error}"),
                    )],
                )),
            }
        }

        let decision = decide(&stages, conflict);

        AdmissionRecord {
            id: format!("audit:{}", proposal.proposal_id),
            proposal_id: proposal.proposal_id,
            decision,
            validation_results: stages,
            changed_verdicts,
            generated_by: proposal.generated_by,
            timestamp: iso8601_now(),
            provenance: know_core::Provenance::default(),
        }
    }

    /// Re-run accepted verdicts against the merged module and diff them
    /// against the base module's verdicts.
    fn check_regression<R: Reasoner>(&self, merged_reasoner: &R) -> (Vec<Diagnostic>, Vec<VerdictDiff>) {
        let mut diagnostics = vec![];
        let mut diffs = vec![];

        let base_reasoner =
            compile::compile(self.base.clone()).ok().and_then(|module| RustdlReasoner::new(&module).ok());

        for check in &self.regression_checks {
            let after = complete_verdict(merged_reasoner.query(&check.proposition));
            let before = base_reasoner.as_ref().and_then(|r| complete_verdict(r.query(&check.proposition)));

            let holds = after.as_ref().is_some_and(|v| check.expected.matches(v));
            if !holds {
                diagnostics.push(Diagnostic::error(
                    codes::ADMISSION_REGRESSION,
                    format!(
                        "regression: '{}' expected {:?}, got {}",
                        check.description,
                        check.expected,
                        after.as_ref().map_or("no complete verdict", |v| v.kind()),
                    ),
                ));
            }

            let changed = match (&before, &after) {
                (Some(b), Some(a)) => b.kind() != a.kind(),
                _ => true,
            };
            if changed {
                diffs.push(VerdictDiff { proposition: check.proposition.to_string(), before, after });
            }
        }

        (diagnostics, diffs)
    }
}

// ---------------------------------------------------------------------------
// Stage implementations
// ---------------------------------------------------------------------------

fn merge(base: &KnowledgeModuleSource, proposal: &KnowledgeProposal) -> KnowledgeModuleSource {
    let mut merged = base.clone();
    merged.concepts.extend(proposal.proposed_concepts.iter().cloned());
    merged.relations.extend(proposal.proposed_relations.iter().cloned());
    merged.entities.extend(proposal.proposed_entities.iter().cloned());
    merged.axioms.extend(proposal.proposed_axioms.iter().cloned());
    merged
}

fn check_lexical(bindings: &[LexicalForm], merged: &KnowledgeModuleSource) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    for form in bindings {
        if form.bindings.is_empty() {
            diagnostics.push(Diagnostic::error(
                codes::INVALID_SENSE_BINDING,
                format!("lexical form '{}' has no concept bindings", form.text),
            ));
        }
        for binding in &form.bindings {
            if !merged.concepts.iter().any(|c| c.id == binding.concept.0) {
                diagnostics.push(Diagnostic::error(
                    codes::INVALID_SENSE_BINDING,
                    format!("lexical form '{}' binds to undeclared concept {}", form.text, binding.concept.0),
                ));
            }
        }
        // A form whose surface text doubles as a concept ID suggests the
        // generator conflated a word with a canonical concept.
        if merged.concepts.iter().any(|c| c.id == form.text) {
            diagnostics.push(Diagnostic::warning(
                codes::INVALID_SENSE_BINDING,
                format!(
                    "surface word '{}' is itself a concept ID; words must bind to \
                     concepts, not be concepts",
                    form.text
                ),
            ));
        }
    }
    diagnostics
}

/// Returns (diagnostics, conflicts_with_existing_knowledge).
fn check_logical<R: Reasoner>(
    reasoner: &R,
    module: &know_ontology::KnowledgeModule,
    proposal: &KnowledgeProposal,
    base: &KnowledgeModuleSource,
) -> (Vec<Diagnostic>, bool) {
    let mut diagnostics = vec![];

    match reasoner.is_consistent() {
        ReasoningOutcome::Complete(true) => {}
        ReasoningOutcome::Complete(false) => {
            diagnostics.push(Diagnostic::error(
                codes::INCONSISTENT_ENTITY,
                "the merged ontology is inconsistent: no model satisfies all axioms".to_string(),
            ));
            return (diagnostics, true);
        }
        ReasoningOutcome::Unsupported(u) => {
            // Unsupported constructs must be explicit errors, not silently
            // admitted knowledge the reasoner cannot check.
            diagnostics.push(Diagnostic::error(
                codes::UNSUPPORTED_FEATURE,
                format!("proposal uses unsupported constructs: {}", u.feature),
            ));
            return (diagnostics, false);
        }
        outcome => {
            diagnostics.push(Diagnostic::error(
                codes::UNSUPPORTED_FEATURE,
                format!("consistency check did not complete: {outcome:?}"),
            ));
            return (diagnostics, false);
        }
    }

    for concept in &proposal.proposed_concepts {
        let id = ConceptId(concept.id.clone());
        let expr = know_ontology::ConceptExpr::Named(id.clone());

        match reasoner.is_satisfiable(&expr) {
            ReasoningOutcome::Complete(true) => {}
            ReasoningOutcome::Complete(false) => {
                diagnostics.push(Diagnostic::error(
                    codes::UNSATISFIABLE_CONCEPT,
                    format!("proposed concept {} can have no instances", concept.id),
                ));
                continue;
            }
            outcome => {
                diagnostics.push(Diagnostic::error(
                    codes::UNSUPPORTED_FEATURE,
                    format!("satisfiability of {} did not complete: {outcome:?}", concept.id),
                ));
                continue;
            }
        }

        // Definitions must not silently collapse distinct concepts: warn when
        // a proposed concept is provably equivalent to a pre-existing one.
        // The native Boolean backend remains complete for this overlap and
        // provides direct-equivalence information that rustdl's adapter does
        // not yet expose as a Know classification result.
        if let ReasoningOutcome::Complete(result) = BooleanReasoner::new(module).classify(&expr) {
            for equivalent in result.equivalent_classes {
                let preexisting = base.concepts.iter().any(|c| c.id == equivalent.0);
                if preexisting {
                    diagnostics.push(Diagnostic::warning(
                        codes::CONCEPT_COLLAPSE,
                        format!(
                            "proposed concept {} is logically equivalent to existing \
                             concept {}; consider extending it instead",
                            concept.id, equivalent.0
                        ),
                    ));
                }
            }
        }
    }

    (diagnostics, false)
}

fn stage(kind: ValidationStage, diagnostics: Vec<Diagnostic>) -> ValidationResult {
    let passed = !diagnostics.iter().any(|d| d.severity == Severity::Error);
    ValidationResult { stage: kind, passed, diagnostics }
}

fn decide(stages: &[ValidationResult], conflict: bool) -> AdmissionDecision {
    let errors: Vec<Diagnostic> =
        stages.iter().flat_map(|s| &s.diagnostics).filter(|d| d.severity == Severity::Error).cloned().collect();
    let warnings: Vec<Diagnostic> =
        stages.iter().flat_map(|s| &s.diagnostics).filter(|d| d.severity == Severity::Warning).cloned().collect();

    if conflict {
        AdmissionDecision::ConflictsWithExistingKnowledge(errors)
    } else if !errors.is_empty() {
        AdmissionDecision::Rejected(errors)
    } else if !warnings.is_empty() {
        AdmissionDecision::AcceptedWithWarnings(warnings)
    } else {
        AdmissionDecision::Accepted
    }
}

fn complete_verdict(outcome: ReasoningOutcome<Verdict>) -> Option<Verdict> {
    match outcome {
        ReasoningOutcome::Complete(v) => Some(v),
        _ => None,
    }
}

fn regression_check(source: RegressionCheckSource) -> RegressionCheck {
    RegressionCheck {
        description: source.description,
        proposition: regression_proposition(source.proposition),
        expected: source.expected,
    }
}

fn regression_proposition(source: RegressionPropositionSource) -> Proposition {
    match source {
        RegressionPropositionSource::ClassMembership { entity, class } => {
            Proposition::ClassMembership { entity: know_core::EntityId(entity), class: expr_from_source(class) }
        }
        RegressionPropositionSource::SubclassOf { child, parent } => {
            Proposition::SubclassOf { child: expr_from_source(child), parent: expr_from_source(parent) }
        }
        RegressionPropositionSource::Equivalent { left, right } => {
            Proposition::Equivalent { left: expr_from_source(left), right: expr_from_source(right) }
        }
        RegressionPropositionSource::Disjoint { left, right } => {
            Proposition::Disjoint { left: expr_from_source(left), right: expr_from_source(right) }
        }
        RegressionPropositionSource::Satisfiable { class } => {
            Proposition::Satisfiable { class: expr_from_source(class) }
        }
        RegressionPropositionSource::RelationHolds { subject, relation, object } => Proposition::RelationHolds {
            subject: know_core::EntityId(subject),
            relation: know_core::RelationId(relation),
            object: know_core::EntityId(object),
        },
        RegressionPropositionSource::Consistent => Proposition::Consistent,
    }
}

fn expr_from_source(source: ConceptExprSource) -> ConceptExpr {
    match source {
        ConceptExprSource::Named(id) => ConceptExpr::Named(ConceptId(id)),
        ConceptExprSource::And(parts) => ConceptExpr::And(parts.into_iter().map(expr_from_source).collect()),
        ConceptExprSource::Or(parts) => ConceptExpr::Or(parts.into_iter().map(expr_from_source).collect()),
        ConceptExprSource::Not(inner) => ConceptExpr::Not(Box::new(expr_from_source(*inner))),
        ConceptExprSource::Exists { relation, filler } => ConceptExpr::Exists {
            relation: know_core::RelationId(relation),
            filler: Box::new(expr_from_source(*filler)),
        },
        ConceptExprSource::ForAll { relation, filler } => ConceptExpr::ForAll {
            relation: know_core::RelationId(relation),
            filler: Box::new(expr_from_source(*filler)),
        },
    }
}
