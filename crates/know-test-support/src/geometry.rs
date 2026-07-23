//! Geometry knowledge fixture — the canonical acceptance suite.
//!
//! Mirrors `knowledge/geometry/concepts.know` as Rust values so tests can
//! build and inspect the module programmatically without touching the filesystem.
//!
//! Required verdicts (architecture plan §16):
//!   Square subclass Rectangle           → Entailed
//!   Square subclass Polygon             → Entailed
//!   Square AND Circle is satisfiable    → Contradicted (unsatisfiable)
//!   Entity classified as Square+Circle  → Inconsistent
//!   Red AND Square subclass Large       → Unknown

use know_ontology::{
    AxiomSource, ConceptExprSource, ConceptRecordSource, ConceptStatus, Grounding,
    KnowledgeModule, KnowledgeModuleSource,
    compile,
};

pub fn geometry_source() -> KnowledgeModuleSource {
    KnowledgeModuleSource {
        id: "geometry".to_string(),
        schema_version: 1,
        concepts: vec![
            concept("geometry::spatial_object", "spatial object", ConceptStatus::Primitive, None),
            concept("geometry::figure",         "figure",         ConceptStatus::Declared,  None),
            concept("geometry::closed_figure",  "closed figure",  ConceptStatus::Declared,  None),
            concept("geometry::polygon",        "polygon",        ConceptStatus::Declared,  None),
            concept("geometry::quadrilateral",  "quadrilateral",  ConceptStatus::Declared,  None),
            concept("geometry::triangle",       "triangle",       ConceptStatus::Declared,  None),
            concept("geometry::rectangle",      "rectangle",      ConceptStatus::Declared,  None),
            concept("geometry::rhombus",        "rhombus",        ConceptStatus::Declared,  None),
            concept(
                "geometry::square",
                "square",
                ConceptStatus::Defined,
                Some(ConceptExprSource::And(vec![
                    ConceptExprSource::Named("geometry::rectangle".into()),
                    ConceptExprSource::Named("geometry::rhombus".into()),
                ])),
            ),
            concept("geometry::circle", "circle", ConceptStatus::Declared, None),
            // Used to demonstrate Unknown verdicts.
            concept("geometry::red",   "red",   ConceptStatus::Primitive, None),
            concept("geometry::large", "large", ConceptStatus::Primitive, None),
        ],
        relations: vec![],
        entities: vec![],
        axioms: vec![
            // Taxonomy (expressed as SubclassOf axioms so the reasoner can
            // compute transitive closure).
            subclass("geometry::figure",        "geometry::spatial_object"),
            subclass("geometry::closed_figure", "geometry::figure"),
            subclass("geometry::polygon",       "geometry::closed_figure"),
            subclass("geometry::quadrilateral", "geometry::polygon"),
            subclass("geometry::triangle",      "geometry::polygon"),
            subclass("geometry::rectangle",     "geometry::quadrilateral"),
            subclass("geometry::rhombus",       "geometry::quadrilateral"),
            subclass("geometry::circle",        "geometry::closed_figure"),
            // Polygon and Circle are disjoint.
            AxiomSource::DisjointClasses {
                classes: vec![
                    ConceptExprSource::Named("geometry::polygon".into()),
                    ConceptExprSource::Named("geometry::circle".into()),
                ],
            },
        ],
    }
}

pub fn geometry_module() -> KnowledgeModule {
    compile::compile(geometry_source()).expect("geometry fixture must always compile")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn concept(
    id: &str,
    label: &str,
    status: ConceptStatus,
    definition: Option<ConceptExprSource>,
) -> ConceptRecordSource {
    ConceptRecordSource {
        id: id.to_string(),
        label: label.to_string(),
        alternate_labels: vec![],
        definition,
        grounding: Some(Grounding::StructuralDefinition),
        status,
        provenance: None,
    }
}

fn subclass(child: &str, parent: &str) -> AxiomSource {
    AxiomSource::SubclassOf {
        child: ConceptExprSource::Named(child.into()),
        parent: ConceptExprSource::Named(parent.into()),
    }
}
