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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    // Deliberately named after the DL complement operator, not std::ops::Not.
    #[allow(clippy::should_implement_trait)]
    pub fn not(expr: ConceptExpr) -> Self {
        Self::Not(Box::new(expr))
    }

    /// True if this expression uses relational restrictions (Exists/ForAll),
    /// which are outside the Boolean fragment.
    pub fn uses_relations(&self) -> bool {
        match self {
            ConceptExpr::Named(_) => false,
            ConceptExpr::And(parts) | ConceptExpr::Or(parts) => {
                parts.iter().any(ConceptExpr::uses_relations)
            }
            ConceptExpr::Not(inner) => inner.uses_relations(),
            ConceptExpr::Exists { .. } | ConceptExpr::ForAll { .. } => true,
        }
    }

    /// Every named concept mentioned anywhere in this expression.
    pub fn named_concepts(&self, out: &mut Vec<ConceptId>) {
        match self {
            ConceptExpr::Named(id) => out.push(id.clone()),
            ConceptExpr::And(parts) | ConceptExpr::Or(parts) => {
                for p in parts {
                    p.named_concepts(out);
                }
            }
            ConceptExpr::Not(inner) => inner.named_concepts(out),
            ConceptExpr::Exists { filler, .. } | ConceptExpr::ForAll { filler, .. } => {
                filler.named_concepts(out)
            }
        }
    }
}

impl std::fmt::Display for ConceptExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn join(
            f: &mut std::fmt::Formatter<'_>,
            parts: &[ConceptExpr],
            op: &str,
        ) -> std::fmt::Result {
            write!(f, "(")?;
            for (i, p) in parts.iter().enumerate() {
                if i > 0 {
                    write!(f, " {op} ")?;
                }
                write!(f, "{p}")?;
            }
            write!(f, ")")
        }

        match self {
            ConceptExpr::Named(id) => write!(f, "{}", id.0),
            ConceptExpr::And(parts) => join(f, parts, "AND"),
            ConceptExpr::Or(parts) => join(f, parts, "OR"),
            ConceptExpr::Not(inner) => write!(f, "(NOT {inner})"),
            ConceptExpr::Exists { relation, filler } => {
                write!(f, "(EXISTS {}.{filler})", relation.0)
            }
            ConceptExpr::ForAll { relation, filler } => {
                write!(f, "(FOR_ALL {}.{filler})", relation.0)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Axioms (IR)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
