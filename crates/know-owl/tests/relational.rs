use know_core::{ConceptId, EntityId, RelationId};
use know_ontology::{
    AxiomSource, ConceptExpr, ConceptExprSource, ConceptRecordSource, ConceptStatus, EntityRecordSource, Grounding,
    KnowledgeModuleSource, RelationRecordSource, compile,
};
use know_owl::RustdlReasoner;
use know_reasoner::{Proposition, Reasoner, ReasoningOutcome, Verdict};

fn named(id: &str) -> ConceptExprSource {
    ConceptExprSource::Named(id.to_string())
}

fn module() -> know_ontology::KnowledgeModule {
    compile::compile(KnowledgeModuleSource {
        id: "family".to_string(),
        schema_version: 1,
        concepts: vec![
            ConceptRecordSource {
                id: "family::person".to_string(),
                label: "person".to_string(),
                alternate_labels: vec![],
                definition: None,
                grounding: Some(Grounding::NaturalKind),
                status: ConceptStatus::Primitive,
                provenance: None,
            },
            ConceptRecordSource {
                id: "family::parent".to_string(),
                label: "parent".to_string(),
                alternate_labels: vec![],
                definition: None,
                grounding: Some(Grounding::Role),
                status: ConceptStatus::Declared,
                provenance: None,
            },
        ],
        relations: vec![RelationRecordSource {
            id: "family::has_child".to_string(),
            label: "has child".to_string(),
            domain: Some(named("family::parent")),
            range: Some(named("family::person")),
            provenance: None,
        }],
        entities: vec![
            EntityRecordSource { id: "family::ada".to_string(), label: "Ada".to_string(), provenance: None },
            EntityRecordSource { id: "family::bea".to_string(), label: "Bea".to_string(), provenance: None },
        ],
        axioms: vec![AxiomSource::RelationAssertion {
            subject: "family::ada".to_string(),
            relation: "family::has_child".to_string(),
            object: "family::bea".to_string(),
        }],
    })
    .expect("family module compiles")
}

fn verdict(reasoner: &RustdlReasoner, proposition: Proposition) -> Verdict {
    match reasoner.query(&proposition) {
        ReasoningOutcome::Complete(verdict) => verdict,
        other => panic!("expected complete result, got {other:?}"),
    }
}

#[test]
fn domain_and_range_axioms_entail_membership_from_relation_assertion() {
    let reasoner = RustdlReasoner::new(&module()).expect("rustdl initializes");

    let parent = verdict(
        &reasoner,
        Proposition::ClassMembership {
            entity: EntityId("family::ada".to_string()),
            class: ConceptExpr::Named(ConceptId("family::parent".to_string())),
        },
    );
    assert!(matches!(parent, Verdict::Entailed(_)), "{parent:?}");

    let person = verdict(
        &reasoner,
        Proposition::ClassMembership {
            entity: EntityId("family::bea".to_string()),
            class: ConceptExpr::Named(ConceptId("family::person".to_string())),
        },
    );
    assert!(matches!(person, Verdict::Entailed(_)), "{person:?}");
}

#[test]
fn asserted_relation_is_entailed() {
    let reasoner = RustdlReasoner::new(&module()).expect("rustdl initializes");
    let result = verdict(
        &reasoner,
        Proposition::RelationHolds {
            subject: EntityId("family::ada".to_string()),
            relation: RelationId("family::has_child".to_string()),
            object: EntityId("family::bea".to_string()),
        },
    );
    assert!(matches!(result, Verdict::Entailed(_)), "{result:?}");
}
