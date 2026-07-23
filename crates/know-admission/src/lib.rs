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

use know_core::{ConceptId, Diagnostic, Provenance, Severity, codes};
use know_lexicon::LexicalForm;
use know_ontology::{
    AxiomSource, ConceptRecordSource, EntityRecordSource, KnowledgeModuleSource,
    RelationRecordSource, compile,
};
use know_reasoner::{
    BooleanReasoner, Proposition, Reasoner, ReasoningOutcome, Verdict,
};
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

// ---------------------------------------------------------------------------
// Regression checks
// ---------------------------------------------------------------------------

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
    fn matches(self, verdict: &Verdict) -> bool {
        matches!(
            (self, verdict),
            (ExpectedVerdict::Entailed, Verdict::Entailed(_))
                | (ExpectedVerdict::Contradicted, Verdict::Contradicted(_))
                | (ExpectedVerdict::Unknown, Verdict::Unknown(_))
        )
    }
}

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
        stages.push(stage(
            ValidationStage::Lexical,
            check_lexical(&proposal.lexical_bindings, &merged_source),
        ));

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
            provenance: Provenance::default(),
        }
    }

    /// Re-run accepted verdicts against the merged module and diff them
    /// against the base module's verdicts.
    fn check_regression(
        &self,
        merged_reasoner: &BooleanReasoner,
    ) -> (Vec<Diagnostic>, Vec<VerdictDiff>) {
        let mut diagnostics = vec![];
        let mut diffs = vec![];

        let base_reasoner = compile::compile(self.base.clone()).ok().map(|m| BooleanReasoner::new(&m));

        for check in &self.regression_checks {
            let after = complete_verdict(merged_reasoner.query(&check.proposition));
            let before = base_reasoner
                .as_ref()
                .and_then(|r| complete_verdict(r.query(&check.proposition)));

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
                diffs.push(VerdictDiff {
                    proposition: check.proposition.to_string(),
                    before,
                    after,
                });
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
                    format!(
                        "lexical form '{}' binds to undeclared concept {}",
                        form.text, binding.concept.0
                    ),
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
    let errors: Vec<Diagnostic> = stages
        .iter()
        .flat_map(|s| &s.diagnostics)
        .filter(|d| d.severity == Severity::Error)
        .cloned()
        .collect();
    let warnings: Vec<Diagnostic> = stages
        .iter()
        .flat_map(|s| &s.diagnostics)
        .filter(|d| d.severity == Severity::Warning)
        .cloned()
        .collect();

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

// ---------------------------------------------------------------------------
// Timestamp (UTC, ISO 8601) without a chrono dependency
// ---------------------------------------------------------------------------

fn iso8601_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    iso8601_from_unix(secs)
}

fn iso8601_from_unix(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // Civil-from-days (Howard Hinnant's algorithm), valid for the Unix era.
    let z = days as i64 + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use know_ontology::{ConceptExprSource, ConceptStatus};
    use know_test_support::geometry::geometry_source;

    fn generator() -> GeneratorIdentity {
        GeneratorIdentity {
            kind: GeneratorKind::Llm,
            model_id: Some("test-model".into()),
            run_id: "run-1".into(),
        }
    }

    fn proposal(id: &str) -> KnowledgeProposal {
        KnowledgeProposal {
            proposal_id: id.into(),
            proposed_concepts: vec![],
            proposed_relations: vec![],
            proposed_entities: vec![],
            proposed_axioms: vec![],
            lexical_bindings: vec![],
            source_evidence: vec![],
            generated_by: generator(),
        }
    }

    fn concept(id: &str, status: ConceptStatus, definition: Option<ConceptExprSource>) -> ConceptRecordSource {
        ConceptRecordSource {
            id: id.into(),
            label: id.into(),
            alternate_labels: vec![],
            definition,
            grounding: None,
            status,
            provenance: None,
        }
    }

    fn pipeline() -> Pipeline {
        Pipeline::new(geometry_source())
    }

    fn decision_errors(decision: &AdmissionDecision) -> &[Diagnostic] {
        match decision {
            AdmissionDecision::Rejected(d)
            | AdmissionDecision::ConflictsWithExistingKnowledge(d)
            | AdmissionDecision::AcceptedWithWarnings(d) => d,
            _ => &[],
        }
    }

    #[test]
    fn valid_proposal_is_accepted() {
        let mut p = proposal("pentagon");
        p.proposed_concepts.push(concept("geometry::pentagon", ConceptStatus::Declared, None));
        p.proposed_axioms.push(AxiomSource::SubclassOf {
            child: ConceptExprSource::Named("geometry::pentagon".into()),
            parent: ConceptExprSource::Named("geometry::polygon".into()),
        });

        let record = pipeline().admit(p);
        assert!(matches!(record.decision, AdmissionDecision::Accepted), "{:?}", record.decision);
        assert!(record.validation_results.iter().all(|s| s.passed));
        assert_eq!(record.validation_results.len(), 5, "all five stages must run");
    }

    #[test]
    fn unresolved_reference_is_rejected() {
        let mut p = proposal("bad-ref");
        p.proposed_axioms.push(AxiomSource::SubclassOf {
            child: ConceptExprSource::Named("geometry::heptagon".into()),
            parent: ConceptExprSource::Named("geometry::polygon".into()),
        });

        let record = pipeline().admit(p);
        let AdmissionDecision::Rejected(diags) = &record.decision else {
            panic!("expected Rejected, got {:?}", record.decision);
        };
        assert!(diags.iter().any(|d| d.code == codes::UNRESOLVED_CONCEPT));
    }

    #[test]
    fn duplicate_concept_id_is_rejected() {
        let mut p = proposal("dup");
        p.proposed_concepts.push(concept("geometry::square", ConceptStatus::Declared, None));

        let record = pipeline().admit(p);
        assert!(
            decision_errors(&record.decision).iter().any(|d| d.code == codes::DUPLICATE_ID),
            "{:?}",
            record.decision
        );
    }

    #[test]
    fn inconsistent_proposal_conflicts_with_existing_knowledge() {
        let mut p = proposal("weird-entity");
        p.proposed_entities.push(EntityRecordSource {
            id: "geometry::weird".into(),
            label: "weird".into(),
            provenance: None,
        });
        p.proposed_axioms.push(AxiomSource::ClassAssertion {
            entity: "geometry::weird".into(),
            class: ConceptExprSource::Named("geometry::square".into()),
        });
        p.proposed_axioms.push(AxiomSource::ClassAssertion {
            entity: "geometry::weird".into(),
            class: ConceptExprSource::Named("geometry::circle".into()),
        });

        let record = pipeline().admit(p);
        assert!(
            matches!(record.decision, AdmissionDecision::ConflictsWithExistingKnowledge(_)),
            "{:?}",
            record.decision
        );
    }

    #[test]
    fn unsatisfiable_proposed_concept_is_rejected() {
        let mut p = proposal("squircle");
        p.proposed_concepts.push(concept(
            "geometry::squircle",
            ConceptStatus::Defined,
            Some(ConceptExprSource::And(vec![
                ConceptExprSource::Named("geometry::square".into()),
                ConceptExprSource::Named("geometry::circle".into()),
            ])),
        ));

        let record = pipeline().admit(p);
        assert!(
            decision_errors(&record.decision).iter().any(|d| d.code == codes::UNSATISFIABLE_CONCEPT),
            "{:?}",
            record.decision
        );
    }

    #[test]
    fn collapsing_definition_is_accepted_with_warning() {
        // Same definition as geometry::square — logically equivalent.
        let mut p = proposal("collapse");
        p.proposed_concepts.push(concept(
            "geometry::equilateral_rectangle",
            ConceptStatus::Defined,
            Some(ConceptExprSource::And(vec![
                ConceptExprSource::Named("geometry::rectangle".into()),
                ConceptExprSource::Named("geometry::rhombus".into()),
            ])),
        ));

        let record = pipeline().admit(p);
        let AdmissionDecision::AcceptedWithWarnings(warnings) = &record.decision else {
            panic!("expected AcceptedWithWarnings, got {:?}", record.decision);
        };
        assert!(warnings.iter().any(|d| {
            d.code == codes::CONCEPT_COLLAPSE && d.message.contains("geometry::square")
        }));
    }

    #[test]
    fn regression_failure_is_rejected_with_verdict_diff() {
        use know_ontology::ConceptExpr;

        // Base invariant: square is satisfiable.
        let checks = vec![RegressionCheck {
            description: "square remains satisfiable".into(),
            proposition: Proposition::Satisfiable {
                class: ConceptExpr::named("geometry::square"),
            },
            expected: ExpectedVerdict::Entailed,
        }];

        // The proposal makes rectangle and rhombus disjoint, so square
        // (their intersection) becomes unsatisfiable.
        let mut p = proposal("bad-disjointness");
        p.proposed_axioms.push(AxiomSource::DisjointClasses {
            classes: vec![
                ConceptExprSource::Named("geometry::rectangle".into()),
                ConceptExprSource::Named("geometry::rhombus".into()),
            ],
        });

        let record = Pipeline::new(geometry_source()).with_regression_checks(checks).admit(p);
        assert!(
            decision_errors(&record.decision).iter().any(|d| d.code == codes::ADMISSION_REGRESSION),
            "{:?}",
            record.decision
        );
        assert_eq!(record.changed_verdicts.len(), 1);
        let diff = &record.changed_verdicts[0];
        assert_eq!(diff.before.as_ref().map(|v| v.kind()), Some("Entailed"));
        assert_eq!(diff.after.as_ref().map(|v| v.kind()), Some("Contradicted"));
    }

    #[test]
    fn lexical_binding_to_missing_concept_is_rejected() {
        use know_core::{ConceptId, LanguageId};
        use know_lexicon::LexicalBinding;

        let mut p = proposal("bad-lexical");
        p.lexical_bindings.push(LexicalForm {
            text: "megagon".into(),
            language: LanguageId::english(),
            part_of_speech: None,
            bindings: vec![LexicalBinding {
                concept: ConceptId("geometry::megagon".into()),
                context_hints: vec![],
                usage_examples: vec![],
                provenance: Provenance::default(),
            }],
        });

        let record = pipeline().admit(p);
        assert!(
            decision_errors(&record.decision).iter().any(|d| d.code == codes::INVALID_SENSE_BINDING),
            "{:?}",
            record.decision
        );
    }

    #[test]
    fn timestamps_are_iso8601() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_from_unix(1_753_228_800), "2025-07-23T00:00:00Z");
    }
}
