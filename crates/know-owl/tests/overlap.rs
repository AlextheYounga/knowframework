use know_core::{AxiomId, ConceptId, EntityId, Provenance};
use know_ontology::{
    AnnotatedAxiom, Axiom, AxiomSource, ConceptExpr, ConceptExprSource, ConceptRecordSource, ConceptStatus,
    EntityRecordSource, Grounding, KnowledgeModuleSource, compile,
};
use know_owl::RustdlReasoner;
use know_reasoner::{BooleanReasoner, Proposition, Reasoner, ReasoningOutcome, Verdict};

fn module() -> Result<know_ontology::KnowledgeModule, Box<dyn Error>> {
    Ok(compile::compile(KnowledgeModuleSource {
        id: "overlap".to_string(),
        schema_version: 1,
        concepts: ["person", "child", "other"]
            .into_iter()
            .map(|id| ConceptRecordSource {
                id: format!("overlap::{id}"),
                label: id.to_string(),
                alternate_labels: vec![],
                definition: None,
                grounding: Some(Grounding::NaturalKind),
                status: ConceptStatus::Primitive,
                provenance: None,
            })
            .collect(),
        relations: vec![],
        entities: vec![EntityRecordSource {
            id: "overlap::ada".to_string(),
            label: "Ada".to_string(),
            provenance: None,
        }],
        axioms: vec![
            AxiomSource::SubclassOf { child: named("overlap::child"), parent: named("overlap::person") },
            AxiomSource::DisjointClasses { classes: vec![named("overlap::person"), named("overlap::other")] },
            AxiomSource::ClassAssertion { entity: "overlap::ada".to_string(), class: named("overlap::child") },
            AxiomSource::NegativeClassAssertion { entity: "overlap::ada".to_string(), class: named("overlap::other") },
        ],
    })?)
}

fn membership_axiom(id: &str, negative: bool, class: &str) -> AnnotatedAxiom {
    let entity = EntityId("overlap::ada".to_string());
    let class = ConceptExpr::Named(ConceptId(class.to_string()));
    AnnotatedAxiom {
        id: AxiomId(id.to_string()),
        axiom: if negative {
            Axiom::NegativeClassAssertion { entity, class }
        } else {
            Axiom::ClassAssertion { entity, class }
        },
        provenance: Provenance::default(),
    }
}

fn named(id: &str) -> ConceptExprSource {
    ConceptExprSource::Named(id.to_string())
}

fn verdict_kind(outcome: ReasoningOutcome<Verdict>) -> &'static str {
    match outcome {
        ReasoningOutcome::Complete(verdict) => verdict.kind(),
        ReasoningOutcome::Unsupported(_) => "Unsupported",
        ReasoningOutcome::ResourceLimit(_) => "ResourceLimit",
        ReasoningOutcome::InternalError(_) => "InternalError",
    }
}

#[test]
fn rustdl_and_boolean_reasoners_agree_on_a_representative_boolean_overlap_matrix() -> Result<(), Box<dyn Error>> {
    let module = module()?;
    let boolean = BooleanReasoner::new(&module);
    let rustdl = RustdlReasoner::new(&module)?;
    let entity = EntityId("overlap::ada".to_string());
    let child = ConceptExpr::Named(ConceptId("overlap::child".to_string()));
    let person = ConceptExpr::Named(ConceptId("overlap::person".to_string()));
    let other = ConceptExpr::Named(ConceptId("overlap::other".to_string()));
    let propositions = vec![
        Proposition::Consistent,
        Proposition::Satisfiable { class: child.clone() },
        Proposition::SubclassOf { child: child.clone(), parent: person.clone() },
        Proposition::Equivalent { left: child, right: person.clone() },
        Proposition::Disjoint { left: person.clone(), right: other.clone() },
        Proposition::ClassMembership { entity: entity.clone(), class: person },
        Proposition::ClassMembership { entity, class: other },
    ];

    for proposition in propositions {
        let expected = verdict_kind(boolean.query(&proposition));
        let actual = verdict_kind(rustdl.query(&proposition));
        assert_eq!(actual, expected, "proposition {proposition}");
    }
    Ok(())
}

#[test]
fn subclass_and_equivalence_distinguish_declared_counterexamples_from_open_cases() -> Result<(), Box<dyn Error>> {
    let mut counterexample = module()?;
    counterexample.axioms = vec![
        membership_axiom("counterexample:child", false, "overlap::child"),
        membership_axiom("counterexample:not-person", true, "overlap::person"),
    ];
    let counterexample_reasoner = RustdlReasoner::new(&counterexample)?;
    let child = ConceptExpr::Named(ConceptId("overlap::child".to_string()));
    let person = ConceptExpr::Named(ConceptId("overlap::person".to_string()));
    for proposition in [
        Proposition::SubclassOf { child: child.clone(), parent: person.clone() },
        Proposition::Equivalent { left: child.clone(), right: person.clone() },
    ] {
        assert!(matches!(
            counterexample_reasoner.query(&proposition),
            ReasoningOutcome::Complete(Verdict::Contradicted(_))
        ));
    }

    let mut open = module()?;
    open.axioms = vec![membership_axiom("open:child", false, "overlap::child")];
    let open_reasoner = RustdlReasoner::new(&open)?;
    let proposition = Proposition::SubclassOf { child, parent: person };
    assert!(matches!(open_reasoner.query(&proposition), ReasoningOutcome::Complete(Verdict::Unknown(_))));
    Ok(())
}

#[test]
fn temporary_constraints_detect_explicit_and_existential_overlap() -> Result<(), Box<dyn Error>> {
    let mut explicit = module()?;
    explicit.axioms = vec![
        membership_axiom("explicit:person", false, "overlap::person"),
        membership_axiom("explicit:other", false, "overlap::other"),
    ];
    let explicit_reasoner = RustdlReasoner::new(&explicit)?;
    assert!(matches!(
        explicit_reasoner.query(&Proposition::Disjoint {
            left: ConceptExpr::Named(ConceptId("overlap::person".to_string())),
            right: ConceptExpr::Named(ConceptId("overlap::other".to_string())),
        }),
        ReasoningOutcome::Complete(Verdict::Contradicted(_))
    ));

    let mut existential = module()?;
    existential.axioms = vec![AnnotatedAxiom {
        id: AxiomId("existential:overlap".to_string()),
        axiom: Axiom::ClassAssertion {
            entity: EntityId("overlap::ada".to_string()),
            class: ConceptExpr::Exists {
                relation: know_core::RelationId("overlap::witness".to_string()),
                filler: Box::new(ConceptExpr::And(vec![
                    ConceptExpr::Named(ConceptId("overlap::person".to_string())),
                    ConceptExpr::Named(ConceptId("overlap::other".to_string())),
                ])),
            },
        },
        provenance: Provenance::default(),
    }];
    existential.relations.push(know_ontology::Relation {
        id: know_core::RelationId("overlap::witness".to_string()),
        preferred_label: "witness".to_string(),
        domain: None,
        range: None,
        provenance: Provenance::default(),
    });
    let reasoner = RustdlReasoner::new(&existential)?;
    let person = ConceptExpr::Named(ConceptId("overlap::person".to_string()));
    let other = ConceptExpr::Named(ConceptId("overlap::other".to_string()));
    let not_other = ConceptExpr::Not(Box::new(other.clone()));
    assert!(matches!(
        reasoner.query(&Proposition::Disjoint { left: person.clone(), right: other.clone() }),
        ReasoningOutcome::Complete(Verdict::Contradicted(_))
    ));
    for proposition in [
        Proposition::SubclassOf { child: person.clone(), parent: not_other.clone() },
        Proposition::Equivalent { left: person, right: not_other },
    ] {
        assert!(matches!(reasoner.query(&proposition), ReasoningOutcome::Complete(Verdict::Contradicted(_))));
    }
    assert!(matches!(
        reasoner.query(&Proposition::ClassMembership { entity: EntityId("overlap::ada".to_string()), class: other }),
        ReasoningOutcome::Complete(Verdict::Unknown(_))
    ));
    Ok(())
}

#[test]
fn open_disjointness_and_inconsistent_ontology_have_distinct_verdicts() -> Result<(), Box<dyn Error>> {
    let mut open = module()?;
    open.axioms.clear();
    let reasoner = RustdlReasoner::new(&open)?;
    let person = ConceptExpr::Named(ConceptId("overlap::person".to_string()));
    let other = ConceptExpr::Named(ConceptId("overlap::other".to_string()));
    assert!(matches!(
        reasoner.query(&Proposition::Disjoint { left: person.clone(), right: other }),
        ReasoningOutcome::Complete(Verdict::Unknown(_))
    ));

    open.axioms = vec![
        membership_axiom("inconsistent:positive", false, "overlap::person"),
        membership_axiom("inconsistent:negative", true, "overlap::person"),
    ];
    let inconsistent = RustdlReasoner::new(&open)?;
    assert!(matches!(
        inconsistent
            .query(&Proposition::ClassMembership { entity: EntityId("overlap::ada".to_string()), class: person }),
        ReasoningOutcome::Complete(Verdict::Inconsistent(_))
    ));
    Ok(())
}
use std::error::Error;
