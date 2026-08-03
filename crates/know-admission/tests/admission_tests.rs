use fixtures::geometry::geometry_source;
use know_admission::{
    AdmissionDecision, ExpectedVerdict, GeneratorIdentity, GeneratorKind, KnowledgeProposal, Pipeline, RegressionCheck,
    RegressionCheckSource, RegressionManifest, RegressionPropositionSource,
};
use know_core::{Diagnostic, Provenance, codes};
use know_ontology::{AxiomSource, ConceptExprSource, ConceptRecordSource, ConceptStatus};

fn generator() -> GeneratorIdentity {
    GeneratorIdentity { kind: GeneratorKind::Llm, model_id: Some("test-model".into()), run_id: "run-1".into() }
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
    assert!(decision_errors(&record.decision).iter().any(|d| d.code == codes::DUPLICATE_ID), "{:?}", record.decision);
}

#[test]
fn inconsistent_proposal_conflicts_with_existing_knowledge() {
    let mut p = proposal("weird-entity");
    p.proposed_entities.push(know_ontology::EntityRecordSource {
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
    assert!(matches!(record.decision, AdmissionDecision::ConflictsWithExistingKnowledge(_)), "{:?}", record.decision);
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
    assert!(warnings.iter().any(|d| { d.code == codes::CONCEPT_COLLAPSE && d.message.contains("geometry::square") }));
}

#[test]
fn regression_failure_is_rejected_with_verdict_diff() {
    use know_ontology::ConceptExpr;

    let checks = vec![RegressionCheck {
        description: "square remains satisfiable".into(),
        proposition: know_reasoner::Proposition::Satisfiable { class: ConceptExpr::named("geometry::square") },
        expected: ExpectedVerdict::Entailed,
    }];

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
    use know_core::LanguageId;
    use know_lexicon::{LexicalBinding, LexicalForm};

    let mut p = proposal("bad-lexical");
    p.lexical_bindings.push(LexicalForm {
        text: "megagon".into(),
        language: LanguageId::english(),
        part_of_speech: None,
        bindings: vec![LexicalBinding {
            concept: know_core::ConceptId("geometry::megagon".into()),
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
fn manifest_regression_is_loaded_and_enforced() {
    let manifest = RegressionManifest {
        checks: vec![RegressionCheckSource {
            description: "square remains satisfiable".into(),
            proposition: RegressionPropositionSource::Satisfiable {
                class: ConceptExprSource::Named("geometry::square".into()),
            },
            expected: ExpectedVerdict::Entailed,
        }],
    };
    let mut p = proposal("bad-disjointness");
    p.proposed_axioms.push(AxiomSource::DisjointClasses {
        classes: vec![
            ConceptExprSource::Named("geometry::rectangle".into()),
            ConceptExprSource::Named("geometry::rhombus".into()),
        ],
    });

    let record = Pipeline::new(geometry_source()).with_regression_manifest(manifest).admit(p);
    assert!(decision_errors(&record.decision).iter().any(|diagnostic| diagnostic.code == codes::ADMISSION_REGRESSION));
}

#[test]
fn apply_returns_a_deterministic_merged_source_only_for_clean_acceptance() {
    let mut p = proposal("pentagon");
    p.proposed_concepts.push(concept("geometry::pentagon", ConceptStatus::Declared, None));
    let pipeline = pipeline();

    let merged = pipeline.apply(p).expect("clean proposal applies");
    assert_eq!(merged.concepts.last().map(|concept| concept.id.as_str()), Some("geometry::pentagon"));
}
