//! Compiles a `KnowledgeModuleSource` into a validated `KnowledgeModule`.
//!
//! Passes performed in order:
//!   1. Symbol collection — gather all declared IDs
//!   2. Duplicate detection
//!   3. Name resolution — resolve all string references to typed IDs
//!   4. Definition-cycle detection
//!
//! Structural validation (satisfiability, consistency, disjointness
//! propagation) is the responsibility of `know-reasoner`, not this module.

use std::collections::HashMap;

use know_core::{AxiomId, ConceptId, Diagnostic, EntityId, ModuleId, Provenance, RelationId, codes};
use thiserror::Error;

use crate::{
    ir::{AnnotatedAxiom, Axiom, Concept, ConceptExpr, Entity, KnowledgeModule, Relation},
    source::{
        AxiomSource, ConceptExprSource, ConceptRecordSource, EntityRecordSource,
        Grounding, KnowledgeModuleSource, RelationRecordSource,
    },
};

pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("unsupported schema version {found}; expected {expected}")]
    UnsupportedSchemaVersion { found: u32, expected: u32 },
    #[error("{count} validation error(s)")]
    ValidationErrors { count: usize, diagnostics: Vec<Diagnostic> },
}

/// Compile a source module into the validated IR.
///
/// Returns `Err` on hard errors (wrong schema version or any unresolved name).
/// Warnings are embedded in diagnostics inside `CompileError::ValidationErrors`.
pub fn compile(source: KnowledgeModuleSource) -> Result<KnowledgeModule, CompileError> {
    if source.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(CompileError::UnsupportedSchemaVersion {
            found: source.schema_version,
            expected: SUPPORTED_SCHEMA_VERSION,
        });
    }

    let mut diagnostics: Vec<Diagnostic> = vec![];

    // --- 1. Collect declared symbols -----------------------------------------

    let concept_ids: HashMap<String, ConceptId> = source
        .concepts
        .iter()
        .map(|c| (c.id.clone(), ConceptId(c.id.clone())))
        .collect();

    let relation_ids: HashMap<String, RelationId> = source
        .relations
        .iter()
        .map(|r| (r.id.clone(), RelationId(r.id.clone())))
        .collect();

    let entity_ids: HashMap<String, EntityId> = source
        .entities
        .iter()
        .map(|e| (e.id.clone(), EntityId(e.id.clone())))
        .collect();

    // --- 2. Duplicate detection -----------------------------------------------

    detect_duplicates(&source.concepts, |c| &c.id, codes::DUPLICATE_ID, &mut diagnostics);
    detect_duplicates(&source.relations, |r| &r.id, codes::DUPLICATE_ID, &mut diagnostics);
    detect_duplicates(&source.entities, |e| &e.id, codes::DUPLICATE_ID, &mut diagnostics);

    // --- 3. Name resolution --------------------------------------------------

    let concepts = source
        .concepts
        .into_iter()
        .map(|c| resolve_concept(c, &concept_ids, &mut diagnostics))
        .collect();

    let relations = source
        .relations
        .into_iter()
        .map(|r| resolve_relation(r, &concept_ids, &mut diagnostics))
        .collect();

    let entities = source
        .entities
        .into_iter()
        .map(|e| resolve_entity(e))
        .collect();

    let axioms = source
        .axioms
        .into_iter()
        .enumerate()
        .map(|(i, a)| resolve_axiom(a, i, &concept_ids, &relation_ids, &entity_ids, &mut diagnostics))
        .collect();

    // --- 4. Definition-cycle detection ---------------------------------------
    //
    // TODO: walk concept definitions and detect cycles using DFS. A cycle
    // through `ConceptExpr::Named` references means the definition is
    // self-referential without an explicit fixpoint semantic, which the current
    // V1 fragment does not support.

    if !diagnostics.iter().any(|d| d.severity == know_core::Severity::Error) {
        Ok(KnowledgeModule {
            id: ModuleId(source.id),
            concepts,
            relations,
            entities,
            axioms,
        })
    } else {
        let count = diagnostics.iter().filter(|d| d.severity == know_core::Severity::Error).count();
        Err(CompileError::ValidationErrors { count, diagnostics })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn detect_duplicates<T, F>(items: &[T], key: F, code: &'static str, out: &mut Vec<Diagnostic>)
where
    F: Fn(&T) -> &String,
{
    let mut seen = HashMap::new();
    for item in items {
        let k = key(item);
        if seen.insert(k.clone(), ()).is_some() {
            out.push(Diagnostic::error(code, format!("duplicate ID: {k}")));
        }
    }
}

fn resolve_expr(
    expr: ConceptExprSource,
    concepts: &HashMap<String, ConceptId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ConceptExpr {
    match expr {
        ConceptExprSource::Named(name) => {
            if let Some(id) = concepts.get(&name) {
                ConceptExpr::Named(id.clone())
            } else {
                diagnostics.push(Diagnostic::error(
                    codes::UNRESOLVED_CONCEPT,
                    format!("unresolved concept: {name}"),
                ));
                // Placeholder so compilation can continue and collect all errors.
                ConceptExpr::Named(ConceptId(name))
            }
        }
        ConceptExprSource::And(parts) => {
            ConceptExpr::And(parts.into_iter().map(|p| resolve_expr(p, concepts, diagnostics)).collect())
        }
        ConceptExprSource::Or(parts) => {
            ConceptExpr::Or(parts.into_iter().map(|p| resolve_expr(p, concepts, diagnostics)).collect())
        }
        ConceptExprSource::Not(inner) => {
            ConceptExpr::Not(Box::new(resolve_expr(*inner, concepts, diagnostics)))
        }
        ConceptExprSource::Exists { relation, filler } => {
            // TODO: resolve relation against the relation namespace once
            // cross-module relation imports are specified. For now, wrap as-is.
            ConceptExpr::Exists {
                relation: RelationId(relation),
                filler: Box::new(resolve_expr(*filler, concepts, diagnostics)),
            }
        }
        ConceptExprSource::ForAll { relation, filler } => {
            ConceptExpr::ForAll {
                relation: RelationId(relation),
                filler: Box::new(resolve_expr(*filler, concepts, diagnostics)),
            }
        }
    }
}

fn resolve_concept(
    src: ConceptRecordSource,
    concepts: &HashMap<String, ConceptId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Concept {
    Concept {
        id: ConceptId(src.id),
        preferred_label: src.label,
        alternate_labels: src.alternate_labels,
        definition: src.definition.map(|d| resolve_expr(d, concepts, diagnostics)),
        grounding: src.grounding.unwrap_or(Grounding::Primitive),
        status: src.status,
        provenance: src.provenance.unwrap_or_default(),
    }
}

fn resolve_relation(
    src: RelationRecordSource,
    concepts: &HashMap<String, ConceptId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Relation {
    Relation {
        id: RelationId(src.id),
        preferred_label: src.label,
        domain: src.domain.map(|d| resolve_expr(d, concepts, diagnostics)),
        range: src.range.map(|r| resolve_expr(r, concepts, diagnostics)),
        provenance: src.provenance.unwrap_or_default(),
    }
}

fn resolve_entity(src: EntityRecordSource) -> Entity {
    Entity {
        id: EntityId(src.id),
        preferred_label: src.label,
        provenance: src.provenance.unwrap_or_default(),
    }
}

fn resolve_entity_ref(
    name: &str,
    entities: &HashMap<String, EntityId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> EntityId {
    if let Some(id) = entities.get(name) {
        id.clone()
    } else {
        diagnostics.push(Diagnostic::error(
            codes::UNRESOLVED_ENTITY,
            format!("unresolved entity: {name}"),
        ));
        EntityId(name.to_string())
    }
}

fn resolve_relation_ref(name: &str, relations: &HashMap<String, RelationId>) -> RelationId {
    // Relations used in axioms may not be declared in this module.
    // TODO: implement cross-module relation resolution.
    relations.get(name).cloned().unwrap_or_else(|| RelationId(name.to_string()))
}

fn resolve_axiom(
    src: AxiomSource,
    index: usize,
    concepts: &HashMap<String, ConceptId>,
    relations: &HashMap<String, RelationId>,
    entities: &HashMap<String, EntityId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> AnnotatedAxiom {
    let axiom_id = AxiomId(format!("axiom:{index}"));

    let axiom = match src {
        AxiomSource::SubclassOf { child, parent } => Axiom::SubclassOf {
            child: resolve_expr(child, concepts, diagnostics),
            parent: resolve_expr(parent, concepts, diagnostics),
        },
        AxiomSource::EquivalentClasses { classes } => Axiom::EquivalentClasses {
            classes: classes.into_iter().map(|e| resolve_expr(e, concepts, diagnostics)).collect(),
        },
        AxiomSource::DisjointClasses { classes } => Axiom::DisjointClasses {
            classes: classes.into_iter().map(|e| resolve_expr(e, concepts, diagnostics)).collect(),
        },
        AxiomSource::ClassAssertion { entity, class } => Axiom::ClassAssertion {
            entity: resolve_entity_ref(&entity, entities, diagnostics),
            class: resolve_expr(class, concepts, diagnostics),
        },
        AxiomSource::RelationAssertion { subject, relation, object } => Axiom::RelationAssertion {
            subject: resolve_entity_ref(&subject, entities, diagnostics),
            relation: resolve_relation_ref(&relation, relations),
            object: resolve_entity_ref(&object, entities, diagnostics),
        },
        AxiomSource::NegativeClassAssertion { entity, class } => Axiom::NegativeClassAssertion {
            entity: resolve_entity_ref(&entity, entities, diagnostics),
            class: resolve_expr(class, concepts, diagnostics),
        },
        AxiomSource::NegativeRelationAssertion { subject, relation, object } => {
            Axiom::NegativeRelationAssertion {
                subject: resolve_entity_ref(&subject, entities, diagnostics),
                relation: resolve_relation_ref(&relation, relations),
                object: resolve_entity_ref(&object, entities, diagnostics),
            }
        }
    };

    AnnotatedAxiom { id: axiom_id, axiom, provenance: Provenance::default() }
}
