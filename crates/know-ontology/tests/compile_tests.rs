use know_core::codes;
use know_ontology::{
    AxiomSource, ConceptExprSource as E, ConceptRecordSource, ConceptStatus, KnowledgeModuleSource,
    compile::{CompileError, compile},
};

fn concept(id: &str, status: ConceptStatus, definition: Option<E>) -> ConceptRecordSource {
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

fn module(concepts: Vec<ConceptRecordSource>, axioms: Vec<AxiomSource>) -> KnowledgeModuleSource {
    KnowledgeModuleSource {
        id: "test".into(),
        schema_version: 1,
        concepts,
        relations: vec![],
        entities: vec![],
        axioms,
    }
}

fn error_codes(err: CompileError) -> Vec<&'static str> {
    err.diagnostics().iter().map(|d| d.code).collect()
}

#[test]
fn compiles_valid_module() {
    let m = module(
        vec![
            concept("a", ConceptStatus::Primitive, None),
            concept("b", ConceptStatus::Defined, Some(E::Named("a".into()))),
        ],
        vec![],
    );
    let compiled = compile(m).expect("must compile");
    assert_eq!(compiled.concepts.len(), 2);
}

#[test]
fn rejects_wrong_schema_version() {
    let mut m = module(vec![], vec![]);
    m.schema_version = 99;
    assert!(matches!(compile(m), Err(CompileError::UnsupportedSchemaVersion { found: 99, .. })));
}

#[test]
fn rejects_duplicate_ids() {
    let m = module(
        vec![concept("a", ConceptStatus::Primitive, None), concept("a", ConceptStatus::Primitive, None)],
        vec![],
    );
    assert_eq!(error_codes(compile(m).unwrap_err()), vec![codes::DUPLICATE_ID]);
}

#[test]
fn rejects_unresolved_concept_reference() {
    let m = module(vec![concept("a", ConceptStatus::Defined, Some(E::Named("missing".into())))], vec![]);
    assert_eq!(error_codes(compile(m).unwrap_err()), vec![codes::UNRESOLVED_CONCEPT]);
}

#[test]
fn rejects_unresolved_relation_in_restriction() {
    let m = module(
        vec![
            concept("a", ConceptStatus::Primitive, None),
            concept(
                "b",
                ConceptStatus::Defined,
                Some(E::Exists { relation: "no_such_relation".into(), filler: Box::new(E::Named("a".into())) }),
            ),
        ],
        vec![],
    );
    assert_eq!(error_codes(compile(m).unwrap_err()), vec![codes::UNRESOLVED_RELATION]);
}

#[test]
fn rejects_unresolved_entity_in_assertion() {
    let m = module(
        vec![concept("a", ConceptStatus::Primitive, None)],
        vec![AxiomSource::ClassAssertion { entity: "ghost".into(), class: E::Named("a".into()) }],
    );
    assert_eq!(error_codes(compile(m).unwrap_err()), vec![codes::UNRESOLVED_ENTITY]);
}

#[test]
fn rejects_defined_concept_without_definition() {
    let m = module(vec![concept("a", ConceptStatus::Defined, None)], vec![]);
    assert_eq!(error_codes(compile(m).unwrap_err()), vec![codes::STATUS_DEFINITION_MISMATCH]);
}

#[test]
fn rejects_primitive_concept_with_definition() {
    let m = module(
        vec![
            concept("a", ConceptStatus::Primitive, None),
            concept("b", ConceptStatus::Primitive, Some(E::Named("a".into()))),
        ],
        vec![],
    );
    assert_eq!(error_codes(compile(m).unwrap_err()), vec![codes::STATUS_DEFINITION_MISMATCH]);
}

#[test]
fn rejects_direct_definition_cycle() {
    let m = module(vec![concept("a", ConceptStatus::Defined, Some(E::Named("a".into())))], vec![]);
    assert_eq!(error_codes(compile(m).unwrap_err()), vec![codes::CIRCULAR_DEFINITION]);
}

#[test]
fn rejects_mutual_definition_cycle() {
    let m = module(
        vec![
            concept("a", ConceptStatus::Defined, Some(E::Named("b".into()))),
            concept("b", ConceptStatus::Defined, Some(E::And(vec![E::Named("a".into())]))),
        ],
        vec![],
    );
    assert_eq!(error_codes(compile(m).unwrap_err()), vec![codes::CIRCULAR_DEFINITION]);
}

#[test]
fn accepts_definition_chain_without_cycle() {
    let m = module(
        vec![
            concept("a", ConceptStatus::Primitive, None),
            concept("b", ConceptStatus::Defined, Some(E::Named("a".into()))),
            concept("c", ConceptStatus::Defined, Some(E::And(vec![E::Named("a".into()), E::Named("b".into())]))),
        ],
        vec![],
    );
    assert!(compile(m).is_ok());
}

#[test]
fn collects_multiple_errors_in_one_pass() {
    let m = module(
        vec![
            concept("a", ConceptStatus::Defined, None),
            concept("b", ConceptStatus::Defined, Some(E::Named("missing".into()))),
        ],
        vec![],
    );
    let codes_found = error_codes(compile(m).unwrap_err());
    assert!(codes_found.contains(&codes::STATUS_DEFINITION_MISMATCH));
    assert!(codes_found.contains(&codes::UNRESOLVED_CONCEPT));
}

#[test]
fn round_trips_through_ron() {
    let m = module(
        vec![
            concept("a", ConceptStatus::Primitive, None),
            concept(
                "b",
                ConceptStatus::Defined,
                Some(E::And(vec![E::Named("a".into()), E::Not(Box::new(E::Named("a".into())))])),
            ),
        ],
        vec![AxiomSource::DisjointClasses { classes: vec![E::Named("a".into()), E::Named("b".into())] }],
    );
    let ron = m.to_ron().expect("serialize");
    let back = KnowledgeModuleSource::from_ron(&ron).expect("deserialize");
    assert_eq!(m, back);
}
