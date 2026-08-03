//! Compiles a `KnowledgeModuleSource` into a validated `KnowledgeModule`.
//!
//! Passes performed in order:
//!   1. Symbol collection — gather all declared IDs
//!   2. Duplicate detection
//!   3. Status/definition consistency
//!   4. Name resolution — resolve all string references to typed IDs
//!   5. Definition-cycle detection
//!
//! All references (concepts, relations, entities) must resolve inside the
//! module being compiled. Cross-module imports are not yet part of the schema;
//! when they are added, symbol collection will consult imported modules first.
//!
//! Semantic validation (satisfiability, consistency, disjointness
//! propagation) is the responsibility of `know-reasoner`, not this module.

use std::collections::{HashMap, HashSet};

use know_core::{AxiomId, ConceptId, Diagnostic, EntityId, ModuleId, Provenance, RelationId, codes};
use thiserror::Error;

use crate::{
    ir::{AnnotatedAxiom, Axiom, Concept, ConceptExpr, Entity, KnowledgeModule, Relation},
    source::{
        AxiomSource, ConceptExprSource, ConceptRecordSource, ConceptStatus, EntityRecordSource, Grounding,
        KnowledgeModuleSource, RelationRecordSource,
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

impl CompileError {
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            CompileError::UnsupportedSchemaVersion { .. } => &[],
            CompileError::ValidationErrors { diagnostics, .. } => diagnostics,
        }
    }
}

struct Symbols {
    concepts: HashMap<String, ConceptId>,
    relations: HashMap<String, RelationId>,
    entities: HashMap<String, EntityId>,
}

/// Compile a source module into the validated IR.
///
/// Returns `Err` on hard errors (wrong schema version or any Error-severity
/// diagnostic). All diagnostics are collected before returning so a single
/// compile reports every problem at once.
pub fn compile(source: KnowledgeModuleSource) -> Result<KnowledgeModule, CompileError> {
    if source.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(CompileError::UnsupportedSchemaVersion {
            found: source.schema_version,
            expected: SUPPORTED_SCHEMA_VERSION,
        });
    }

    let mut diagnostics: Vec<Diagnostic> = vec![];

    // --- 1. Collect declared symbols ---------------------------------------

    let symbols = Symbols {
        concepts: source.concepts.iter().map(|c| (c.id.clone(), ConceptId(c.id.clone()))).collect(),
        relations: source.relations.iter().map(|r| (r.id.clone(), RelationId(r.id.clone()))).collect(),
        entities: source.entities.iter().map(|e| (e.id.clone(), EntityId(e.id.clone()))).collect(),
    };

    // --- 2. Duplicate detection ---------------------------------------------

    detect_duplicates(&source.concepts, |c| &c.id, &mut diagnostics);
    detect_duplicates(&source.relations, |r| &r.id, &mut diagnostics);
    detect_duplicates(&source.entities, |e| &e.id, &mut diagnostics);

    // --- 3. Status/definition consistency -----------------------------------

    for c in &source.concepts {
        check_status_definition(c, &mut diagnostics);
    }

    // --- 4. Name resolution --------------------------------------------------

    let concepts: Vec<Concept> =
        source.concepts.into_iter().map(|c| resolve_concept(c, &symbols, &mut diagnostics)).collect();

    let relations = source.relations.into_iter().map(|r| resolve_relation(r, &symbols, &mut diagnostics)).collect();

    let entities = source.entities.into_iter().map(resolve_entity).collect();

    let axioms =
        source.axioms.into_iter().enumerate().map(|(i, a)| resolve_axiom(a, i, &symbols, &mut diagnostics)).collect();

    // --- 5. Definition-cycle detection ---------------------------------------

    detect_definition_cycles(&concepts, &mut diagnostics);

    let error_count = diagnostics.iter().filter(|d| d.severity == know_core::Severity::Error).count();

    if error_count == 0 {
        Ok(KnowledgeModule { id: ModuleId(source.id), concepts, relations, entities, axioms })
    } else {
        Err(CompileError::ValidationErrors { count: error_count, diagnostics })
    }
}

// ---------------------------------------------------------------------------
// Pass 2: duplicates
// ---------------------------------------------------------------------------

fn detect_duplicates<T, F>(items: &[T], key: F, out: &mut Vec<Diagnostic>)
where
    F: Fn(&T) -> &String,
{
    let mut seen = HashSet::new();
    for item in items {
        let k = key(item);
        if !seen.insert(k.clone()) {
            out.push(Diagnostic::error(codes::DUPLICATE_ID, format!("duplicate ID: {k}")));
        }
    }
}

// ---------------------------------------------------------------------------
// Pass 3: status/definition consistency
//
// `Defined` means "has necessary and sufficient conditions", so a definition
// is mandatory. `Primitive` and `Declared` explicitly mean "no complete
// definition", so one must not be present — a definition appearing on them
// is a sign the status is wrong, not that the definition is extra data.
// `Deprecated` concepts keep whatever they had for version history.
// ---------------------------------------------------------------------------

fn check_status_definition(c: &ConceptRecordSource, out: &mut Vec<Diagnostic>) {
    match (c.status, c.definition.is_some()) {
        (ConceptStatus::Defined, false) => out.push(Diagnostic::error(
            codes::STATUS_DEFINITION_MISMATCH,
            format!("concept {} has status Defined but no definition", c.id),
        )),
        (ConceptStatus::Primitive | ConceptStatus::Declared, true) => out.push(Diagnostic::error(
            codes::STATUS_DEFINITION_MISMATCH,
            format!("concept {} has status {:?} but carries a definition; use status Defined", c.id, c.status),
        )),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Pass 4: name resolution
// ---------------------------------------------------------------------------

fn resolve_expr(expr: ConceptExprSource, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) -> ConceptExpr {
    match expr {
        ConceptExprSource::Named(name) => {
            if !symbols.concepts.contains_key(&name) {
                diagnostics.push(Diagnostic::error(codes::UNRESOLVED_CONCEPT, format!("unresolved concept: {name}")));
            }
            // On error the placeholder ID lets compilation continue and
            // collect further diagnostics; the module is discarded anyway.
            ConceptExpr::Named(ConceptId(name))
        }
        ConceptExprSource::And(parts) => {
            ConceptExpr::And(parts.into_iter().map(|p| resolve_expr(p, symbols, diagnostics)).collect())
        }
        ConceptExprSource::Or(parts) => {
            ConceptExpr::Or(parts.into_iter().map(|p| resolve_expr(p, symbols, diagnostics)).collect())
        }
        ConceptExprSource::Not(inner) => ConceptExpr::Not(Box::new(resolve_expr(*inner, symbols, diagnostics))),
        ConceptExprSource::Exists { relation, filler } => ConceptExpr::Exists {
            relation: resolve_relation_ref(&relation, symbols, diagnostics),
            filler: Box::new(resolve_expr(*filler, symbols, diagnostics)),
        },
        ConceptExprSource::ForAll { relation, filler } => ConceptExpr::ForAll {
            relation: resolve_relation_ref(&relation, symbols, diagnostics),
            filler: Box::new(resolve_expr(*filler, symbols, diagnostics)),
        },
    }
}

fn resolve_concept(src: ConceptRecordSource, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) -> Concept {
    Concept {
        id: ConceptId(src.id),
        preferred_label: src.label,
        alternate_labels: src.alternate_labels,
        definition: src.definition.map(|d| resolve_expr(d, symbols, diagnostics)),
        grounding: src.grounding.unwrap_or(Grounding::Primitive),
        status: src.status,
        provenance: src.provenance.unwrap_or_default(),
    }
}

fn resolve_relation(src: RelationRecordSource, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) -> Relation {
    Relation {
        id: RelationId(src.id),
        preferred_label: src.label,
        domain: src.domain.map(|d| resolve_expr(d, symbols, diagnostics)),
        range: src.range.map(|r| resolve_expr(r, symbols, diagnostics)),
        provenance: src.provenance.unwrap_or_default(),
    }
}

fn resolve_entity(src: EntityRecordSource) -> Entity {
    Entity { id: EntityId(src.id), preferred_label: src.label, provenance: src.provenance.unwrap_or_default() }
}

fn resolve_entity_ref(name: &str, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) -> EntityId {
    if !symbols.entities.contains_key(name) {
        diagnostics.push(Diagnostic::error(codes::UNRESOLVED_ENTITY, format!("unresolved entity: {name}")));
    }
    EntityId(name.to_string())
}

fn resolve_relation_ref(name: &str, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) -> RelationId {
    if !symbols.relations.contains_key(name) {
        diagnostics.push(Diagnostic::error(codes::UNRESOLVED_RELATION, format!("unresolved relation: {name}")));
    }
    RelationId(name.to_string())
}

fn resolve_axiom(
    src: AxiomSource,
    index: usize,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) -> AnnotatedAxiom {
    let axiom_id = AxiomId(format!("axiom:{index}"));

    let axiom = match src {
        AxiomSource::SubclassOf { child, parent } => Axiom::SubclassOf {
            child: resolve_expr(child, symbols, diagnostics),
            parent: resolve_expr(parent, symbols, diagnostics),
        },
        AxiomSource::EquivalentClasses { classes } => Axiom::EquivalentClasses {
            classes: classes.into_iter().map(|e| resolve_expr(e, symbols, diagnostics)).collect(),
        },
        AxiomSource::DisjointClasses { classes } => Axiom::DisjointClasses {
            classes: classes.into_iter().map(|e| resolve_expr(e, symbols, diagnostics)).collect(),
        },
        AxiomSource::ClassAssertion { entity, class } => Axiom::ClassAssertion {
            entity: resolve_entity_ref(&entity, symbols, diagnostics),
            class: resolve_expr(class, symbols, diagnostics),
        },
        AxiomSource::RelationAssertion { subject, relation, object } => Axiom::RelationAssertion {
            subject: resolve_entity_ref(&subject, symbols, diagnostics),
            relation: resolve_relation_ref(&relation, symbols, diagnostics),
            object: resolve_entity_ref(&object, symbols, diagnostics),
        },
        AxiomSource::NegativeClassAssertion { entity, class } => Axiom::NegativeClassAssertion {
            entity: resolve_entity_ref(&entity, symbols, diagnostics),
            class: resolve_expr(class, symbols, diagnostics),
        },
        AxiomSource::NegativeRelationAssertion { subject, relation, object } => Axiom::NegativeRelationAssertion {
            subject: resolve_entity_ref(&subject, symbols, diagnostics),
            relation: resolve_relation_ref(&relation, symbols, diagnostics),
            object: resolve_entity_ref(&object, symbols, diagnostics),
        },
    };

    AnnotatedAxiom { id: axiom_id, axiom, provenance: Provenance::default() }
}

// ---------------------------------------------------------------------------
// Pass 5: definition cycles
//
// A concept definition may reference other defined concepts, but the chain
// must bottom out at primitives. A cycle (square = ... square ...) has no
// fixpoint semantics in the V1 fragment, so it is rejected.
//
// Only edges through `definition` count: subclass axioms between concepts
// form hierarchies, not definitional substitution, and may safely be
// "cyclic" (mutual subclass = equivalence).
// ---------------------------------------------------------------------------

fn detect_definition_cycles(concepts: &[Concept], out: &mut Vec<Diagnostic>) {
    let defined: HashMap<&ConceptId, &ConceptExpr> =
        concepts.iter().filter_map(|c| c.definition.as_ref().map(|d| (&c.id, d))).collect();

    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        InProgress,
        Done,
    }

    let mut marks: HashMap<&ConceptId, Mark> = HashMap::new();

    fn visit<'a>(
        id: &'a ConceptId,
        defined: &HashMap<&'a ConceptId, &'a ConceptExpr>,
        marks: &mut HashMap<&'a ConceptId, Mark>,
        stack: &mut Vec<&'a ConceptId>,
        out: &mut Vec<Diagnostic>,
    ) {
        match marks.get(id) {
            Some(Mark::Done) => return,
            Some(Mark::InProgress) => {
                let cycle_start = stack.iter().position(|c| *c == id).unwrap_or(0);
                let cycle: Vec<&str> =
                    stack[cycle_start..].iter().map(|c| c.0.as_str()).chain(std::iter::once(id.0.as_str())).collect();
                out.push(Diagnostic::error(
                    codes::CIRCULAR_DEFINITION,
                    format!("circular definition: {}", cycle.join(" -> ")),
                ));
                return;
            }
            None => {}
        }

        let Some(expr) = defined.get(id) else {
            marks.insert(id, Mark::Done);
            return;
        };

        marks.insert(id, Mark::InProgress);
        stack.push(id);

        let mut referenced = vec![];
        expr.named_concepts(&mut referenced);
        for r in &referenced {
            // Resolve the reference back to a key owned by `defined`.
            if let Some((key, _)) = defined.get_key_value(r) {
                visit(key, defined, marks, stack, out);
            }
        }

        stack.pop();
        marks.insert(id, Mark::Done);
    }

    let keys: Vec<&ConceptId> = defined.keys().copied().collect();
    for id in keys {
        let mut stack = vec![];
        visit(id, &defined, &mut marks, &mut stack, out);
    }
}

