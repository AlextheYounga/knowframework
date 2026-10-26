use std::{error::Error, io};

use know_core::{AxiomId, ConceptId, EntityId, Provenance, RelationId};
use know_ontology::{
    AnnotatedAxiom, Axiom, AxiomSource, ConceptExpr, ConceptExprSource, ConceptRecordSource, ConceptStatus,
    EntityRecordSource, Grounding, KnowledgeModuleSource, RelationRecordSource, compile,
};
use know_owl::{OwlTranslationError, RustdlInitError, RustdlReasoner, export_owl_functional};
use know_reasoner::{Proposition, Reasoner, ReasoningOutcome, Verdict};

fn named(id: &str) -> ConceptExprSource {
    ConceptExprSource::Named(id.to_string())
}

fn module() -> Result<know_ontology::KnowledgeModule, Box<dyn Error>> {
    Ok(compile::compile(KnowledgeModuleSource {
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
    })?)
}

fn verdict(reasoner: &RustdlReasoner, proposition: &Proposition) -> Result<Verdict, Box<dyn Error>> {
    match reasoner.query(proposition) {
        ReasoningOutcome::Complete(verdict) => Ok(verdict),
        other => Err(io::Error::other(format!("expected complete result, got {other:?}")).into()),
    }
}

fn nested_not(class: &str) -> ConceptExpr {
    ConceptExpr::Not(Box::new(ConceptExpr::Not(Box::new(ConceptExpr::Named(ConceptId(class.to_string()))))))
}

#[test]
fn domain_and_range_axioms_entail_membership_from_relation_assertion() -> Result<(), Box<dyn Error>> {
    let reasoner = RustdlReasoner::new(&module()?)?;

    let parent = verdict(
        &reasoner,
        &Proposition::ClassMembership {
            entity: EntityId("family::ada".to_string()),
            class: ConceptExpr::Named(ConceptId("family::parent".to_string())),
        },
    )?;
    assert!(matches!(parent, Verdict::Entailed(_)), "{parent:?}");

    let person = verdict(
        &reasoner,
        &Proposition::ClassMembership {
            entity: EntityId("family::bea".to_string()),
            class: ConceptExpr::Named(ConceptId("family::person".to_string())),
        },
    )?;
    assert!(matches!(person, Verdict::Entailed(_)), "{person:?}");
    Ok(())
}

#[test]
fn asserted_relation_is_entailed() -> Result<(), Box<dyn Error>> {
    let reasoner = RustdlReasoner::new(&module()?)?;
    let result = verdict(
        &reasoner,
        &Proposition::RelationHolds {
            subject: EntityId("family::ada".to_string()),
            relation: RelationId("family::has_child".to_string()),
            object: EntityId("family::bea".to_string()),
        },
    )?;
    assert!(matches!(result, Verdict::Entailed(_)), "{result:?}");
    Ok(())
}

#[test]
fn exports_negative_assertions_and_all_first_slice_expression_forms() -> Result<(), Box<dyn Error>> {
    let mut module = module()?;
    let person = ConceptExpr::Named(ConceptId("family::person".to_string()));
    let parent = ConceptExpr::Named(ConceptId("family::parent".to_string()));
    let all_forms = ConceptExpr::And(vec![
        person.clone(),
        ConceptExpr::Or(vec![parent.clone(), ConceptExpr::Not(Box::new(person.clone()))]),
        ConceptExpr::Exists { relation: RelationId("family::has_child".to_string()), filler: Box::new(person) },
        ConceptExpr::ForAll { relation: RelationId("family::has_child".to_string()), filler: Box::new(parent) },
    ]);
    module.axioms.extend([
        AnnotatedAxiom {
            id: AxiomId("test:negative-class".to_string()),
            axiom: Axiom::NegativeClassAssertion {
                entity: EntityId("family::ada".to_string()),
                class: all_forms.clone(),
            },
            provenance: Provenance::default(),
        },
        AnnotatedAxiom {
            id: AxiomId("test:negative-relation".to_string()),
            axiom: Axiom::NegativeRelationAssertion {
                subject: EntityId("family::bea".to_string()),
                relation: RelationId("family::has_child".to_string()),
                object: EntityId("family::ada".to_string()),
            },
            provenance: Provenance::default(),
        },
    ]);

    let owl = export_owl_functional(&module)?;

    assert!(owl.contains("ClassAssertion(ObjectComplementOf(ObjectIntersectionOf("), "{owl}");
    assert!(owl.contains("NegativeObjectPropertyAssertion("), "{owl}");
    assert!(owl.contains("ObjectUnionOf("), "{owl}");
    assert!(owl.contains("ObjectSomeValuesFrom("), "{owl}");
    assert!(owl.contains("ObjectAllValuesFrom("), "{owl}");
    let reasoner = RustdlReasoner::new(&module)?;
    let negative_class = verdict(
        &reasoner,
        &Proposition::ClassMembership { entity: EntityId("family::ada".to_string()), class: all_forms },
    )?;
    assert!(matches!(negative_class, Verdict::Contradicted(_)), "{negative_class:?}");
    let negative_relation = verdict(
        &reasoner,
        &Proposition::RelationHolds {
            subject: EntityId("family::bea".to_string()),
            relation: RelationId("family::has_child".to_string()),
            object: EntityId("family::ada".to_string()),
        },
    )?;
    assert!(matches!(negative_relation, Verdict::Contradicted(_)), "{negative_relation:?}");
    Ok(())
}

#[test]
fn constructor_rejects_unknown_references_before_reasoning() -> Result<(), Box<dyn Error>> {
    let mut invalid = module()?;
    invalid.axioms.push(AnnotatedAxiom {
        id: AxiomId("test:unknown-concept".to_string()),
        axiom: Axiom::ClassAssertion {
            entity: EntityId("family::ada".to_string()),
            class: ConceptExpr::Named(ConceptId("family::missing".to_string())),
        },
        provenance: Provenance::default(),
    });

    let Err(error) = RustdlReasoner::new(&invalid) else {
        return Err(io::Error::other("unknown input was accepted").into());
    };
    assert!(matches!(error, RustdlInitError::Unsupported(message) if message.contains("unknown concept")));
    Ok(())
}

#[test]
fn constructor_rejects_duplicate_source_axiom_ids_before_backend_execution() -> Result<(), Box<dyn Error>> {
    let mut invalid = module()?;
    let duplicate = invalid.axioms[0].clone();
    invalid.axioms.push(duplicate);

    let Err(error) = RustdlReasoner::new(&invalid) else {
        return Err(io::Error::other("duplicate source axiom IDs were accepted").into());
    };
    assert!(matches!(error, RustdlInitError::Unsupported(message) if message.contains("duplicate source axiom ID")));
    assert!(
        matches!(export_owl_functional(&invalid), Err(OwlTranslationError::InvalidStructure(message)) if message.contains("duplicate source axiom ID"))
    );
    Ok(())
}

#[test]
fn translated_coverage_keeps_relation_records_out_of_explanation_support() -> Result<(), Box<dyn Error>> {
    let reasoner = RustdlReasoner::new(&module()?)?;
    assert_eq!(reasoner.translated_domain_range_count(), 2);
    assert_eq!(reasoner.translated_axioms().len(), 1);
    assert!(reasoner.translated_axioms().contains_key(&AxiomId("axiom:0".to_string())));

    let result = verdict(
        &reasoner,
        &Proposition::ClassMembership {
            entity: EntityId("family::ada".to_string()),
            class: ConceptExpr::Named(ConceptId("family::parent".to_string())),
        },
    )?;
    let Verdict::Entailed(explanation) = result else {
        return Err(io::Error::other("domain inference should be entailed").into());
    };
    assert!(explanation.supporting_axioms.is_empty());
    assert!(explanation.steps.is_empty());
    Ok(())
}

#[test]
fn backend_normalizes_nested_not_at_declaration_axiom_and_probe_boundaries() -> Result<(), Box<dyn Error>> {
    let mut module = module()?;
    module.concepts[0].definition = Some(nested_not("family::parent"));
    module.relations[0].domain = Some(nested_not("family::parent"));
    module.relations[0].range = Some(nested_not("family::person"));
    module.axioms.extend([
        AnnotatedAxiom {
            id: AxiomId("test:nested-subclass".to_string()),
            axiom: Axiom::SubclassOf { child: nested_not("family::parent"), parent: nested_not("family::person") },
            provenance: Provenance::default(),
        },
        AnnotatedAxiom {
            id: AxiomId("test:nested-equivalent".to_string()),
            axiom: Axiom::EquivalentClasses {
                classes: vec![nested_not("family::parent"), nested_not("family::parent")],
            },
            provenance: Provenance::default(),
        },
        AnnotatedAxiom {
            id: AxiomId("test:nested-disjoint".to_string()),
            axiom: Axiom::DisjointClasses {
                classes: vec![
                    nested_not("family::parent"),
                    ConceptExpr::Not(Box::new(ConceptExpr::Named(ConceptId("family::parent".to_string())))),
                ],
            },
            provenance: Provenance::default(),
        },
        AnnotatedAxiom {
            id: AxiomId("test:nested-positive-assertion".to_string()),
            axiom: Axiom::ClassAssertion {
                entity: EntityId("family::ada".to_string()),
                class: nested_not("family::parent"),
            },
            provenance: Provenance::default(),
        },
    ]);

    let reasoner = RustdlReasoner::new(&module)?;

    for proposition in [
        Proposition::Satisfiable { class: nested_not("family::parent") },
        Proposition::ClassMembership {
            entity: EntityId("family::ada".to_string()),
            class: nested_not("family::parent"),
        },
        Proposition::SubclassOf { child: nested_not("family::parent"), parent: nested_not("family::person") },
        Proposition::Equivalent { left: nested_not("family::parent"), right: nested_not("family::parent") },
        Proposition::Disjoint {
            left: nested_not("family::parent"),
            right: ConceptExpr::Not(Box::new(ConceptExpr::Named(ConceptId("family::parent".to_string())))),
        },
    ] {
        let result = verdict(&reasoner, &proposition)?;
        assert!(matches!(result, Verdict::Entailed(_)), "nested NNF probe {proposition:?}: {result:?}");
    }
    Ok(())
}

#[test]
fn export_normalizes_empty_and_singleton_boolean_expressions_and_rejects_undersized_class_axioms()
-> Result<(), Box<dyn Error>> {
    let mut valid = module()?;
    valid.axioms.extend([
        AnnotatedAxiom {
            id: AxiomId("test:empty-and".to_string()),
            axiom: Axiom::ClassAssertion {
                entity: EntityId("family::ada".to_string()),
                class: ConceptExpr::And(vec![]),
            },
            provenance: Provenance::default(),
        },
        AnnotatedAxiom {
            id: AxiomId("test:singleton-or".to_string()),
            axiom: Axiom::ClassAssertion {
                entity: EntityId("family::ada".to_string()),
                class: ConceptExpr::Or(vec![ConceptExpr::Named(ConceptId("family::person".to_string()))]),
            },
            provenance: Provenance::default(),
        },
    ]);
    let owl = export_owl_functional(&valid)?;
    assert!(owl.contains("<http://www.w3.org/2002/07/owl#Thing>"), "{owl}");
    assert!(!owl.contains("ObjectUnionOf(<urn:know:concept:66616d696c793a3a706572736f6e>)"), "{owl}");

    for axiom in [
        Axiom::EquivalentClasses { classes: vec![] },
        Axiom::DisjointClasses { classes: vec![ConceptExpr::Named(ConceptId("family::person".to_string()))] },
    ] {
        let mut invalid = module()?;
        invalid.axioms.push(AnnotatedAxiom {
            id: AxiomId("test:invalid-nary".to_string()),
            axiom,
            provenance: Provenance::default(),
        });
        assert!(matches!(export_owl_functional(&invalid), Err(OwlTranslationError::InvalidStructure(_))));
    }
    Ok(())
}

#[test]
fn backend_executes_empty_singleton_and_complement_boolean_expressions() -> Result<(), Box<dyn Error>> {
    let mut module = module()?;
    let person = ConceptExpr::Named(ConceptId("family::person".to_string()));
    module.axioms.push(AnnotatedAxiom {
        id: AxiomId("test:empty-and-membership".to_string()),
        axiom: Axiom::ClassAssertion { entity: EntityId("family::ada".to_string()), class: ConceptExpr::And(vec![]) },
        provenance: Provenance::default(),
    });
    let reasoner = RustdlReasoner::new(&module)?;

    for (class, expected) in [
        (ConceptExpr::And(vec![]), "Entailed"),
        (ConceptExpr::Or(vec![]), "Contradicted"),
        (ConceptExpr::And(vec![person.clone()]), "Entailed"),
        (ConceptExpr::Or(vec![person]), "Entailed"),
        (ConceptExpr::Not(Box::new(ConceptExpr::And(vec![]))), "Contradicted"),
        (ConceptExpr::Not(Box::new(ConceptExpr::Or(vec![]))), "Entailed"),
    ] {
        let result = verdict(&reasoner, &Proposition::Satisfiable { class })?;
        assert_eq!(result.kind(), expected, "backend Boolean identity result was {result:?}");
    }
    let membership = verdict(
        &reasoner,
        &Proposition::ClassMembership { entity: EntityId("family::ada".to_string()), class: ConceptExpr::And(vec![]) },
    )?;
    assert!(matches!(membership, Verdict::Entailed(_)), "{membership:?}");
    Ok(())
}
