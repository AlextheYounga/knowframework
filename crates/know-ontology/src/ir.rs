//! Validated intermediate representation.
//!
//! All string names have been resolved to typed IDs. Code in `know-reasoner`
//! and `know-admission` operates exclusively on this representation.

use know_core::{AxiomId, ConceptId, EntityId, Provenance, RelationId};
use serde::{Deserialize, Serialize};

use crate::source::{ConceptStatus, Grounding};

// ---------------------------------------------------------------------------
// Concept expressions (IR — all names resolved to typed IDs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConceptExpr {
    Named(ConceptId),
    And(Vec<ConceptExpr>),
    Or(Vec<ConceptExpr>),
    Not(Box<ConceptExpr>),
    Exists { relation: RelationId, filler: Box<ConceptExpr> },
    ForAll { relation: RelationId, filler: Box<ConceptExpr> },
}

impl ConceptExpr {
    pub fn named(id: impl Into<String>) -> Self {
        Self::Named(ConceptId(id.into()))
    }

    pub fn and(exprs: Vec<ConceptExpr>) -> Self {
        Self::And(exprs)
    }

    pub fn or(exprs: Vec<ConceptExpr>) -> Self {
        Self::Or(exprs)
    }

    pub fn not(expr: ConceptExpr) -> Self {
        Self::Not(Box::new(expr))
    }
}

// ---------------------------------------------------------------------------
// Axioms (IR)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Axiom {
    SubclassOf {
        child: ConceptExpr,
        parent: ConceptExpr,
    },
    EquivalentClasses {
        classes: Vec<ConceptExpr>,
    },
    DisjointClasses {
        classes: Vec<ConceptExpr>,
    },
    ClassAssertion {
        entity: EntityId,
        class: ConceptExpr,
    },
    RelationAssertion {
        subject: EntityId,
        relation: RelationId,
        object: EntityId,
    },
    NegativeClassAssertion {
        entity: EntityId,
        class: ConceptExpr,
    },
    NegativeRelationAssertion {
        subject: EntityId,
        relation: RelationId,
        object: EntityId,
    },
}

#[derive(Debug, Clone)]
pub struct AnnotatedAxiom {
    pub id: AxiomId,
    pub axiom: Axiom,
    pub provenance: Provenance,
}

// ---------------------------------------------------------------------------
// Domain records (IR)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Concept {
    pub id: ConceptId,
    pub preferred_label: String,
    pub alternate_labels: Vec<String>,
    pub definition: Option<ConceptExpr>,
    pub grounding: Grounding,
    pub status: ConceptStatus,
    pub provenance: Provenance,
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub id: RelationId,
    pub preferred_label: String,
    pub domain: Option<ConceptExpr>,
    pub range: Option<ConceptExpr>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: EntityId,
    pub preferred_label: String,
    pub provenance: Provenance,
}

// ---------------------------------------------------------------------------
// Knowledge module (IR)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct KnowledgeModule {
    pub id: know_core::ModuleId,
    pub concepts: Vec<Concept>,
    pub relations: Vec<Relation>,
    pub entities: Vec<Entity>,
    pub axioms: Vec<AnnotatedAxiom>,
}

impl KnowledgeModule {
    pub fn concept(&self, id: &ConceptId) -> Option<&Concept> {
        self.concepts.iter().find(|c| &c.id == id)
    }

    pub fn relation(&self, id: &RelationId) -> Option<&Relation> {
        self.relations.iter().find(|r| &r.id == id)
    }

    pub fn entity(&self, id: &EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| &e.id == id)
    }
}
