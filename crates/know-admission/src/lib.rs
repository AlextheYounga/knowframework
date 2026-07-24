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
use know_ontology::{KnowledgeModuleSource, compile};
use know_reasoner::{BooleanReasoner, Reasoner, ReasoningOutcome, Verdict};

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
            let reasoner = BooleanReasoner::new(module);

            let (logical_diags, is_conflict) = check_logical(&reasoner, &proposal, &self.base);
            conflict = is_conflict;
            stages.push(stage(ValidationStage::Logical, logical_diags));

            if !conflict {
                let (regression_diags, diffs) = self.check_regression(&reasoner);
                changed_verdicts = diffs;
                stages.push(stage(ValidationStage::Regression, regression_diags));
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
    fn check_regression(&self, merged_reasoner: &BooleanReasoner) -> (Vec<Diagnostic>, Vec<VerdictDiff>) {
        let mut diagnostics = vec![];
        let mut diffs = vec![];

        let base_reasoner = compile::compile(self.base.clone()).ok().map(|m| BooleanReasoner::new(&m));

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
fn check_logical(
    reasoner: &BooleanReasoner,
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
        if let ReasoningOutcome::Complete(result) = reasoner.classify(&expr) {
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
