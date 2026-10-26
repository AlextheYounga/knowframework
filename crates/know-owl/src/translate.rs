use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Cursor;

use horned_owl::io::ParserConfiguration;
use horned_owl::io::ofn::reader::read as read_ofn;
use horned_owl::model::RcStr;
use horned_owl::ontology::set::SetOntology;
use know_core::{AxiomId, ConceptId, EntityId, RelationId};
use know_ontology::{Axiom, ConceptExpr, KnowledgeModule};

use crate::iri::{OWL_NOTHING, OWL_THING, concept_iri, entity_iri, relation_iri};

/// Parser-validated OWL 2 Functional Syntax and its standards source metadata.
#[derive(Debug, Clone)]
pub struct OwlTranslation {
    functional_syntax: String,
    source_axioms: HashMap<AxiomId, OwlAxiom>,
    domain_range_coverage: usize,
}

impl OwlTranslation {
    /// The complete OWL 2 Functional Syntax ontology.
    #[must_use]
    pub fn functional_syntax(&self) -> &str {
        &self.functional_syntax
    }

    /// Standards OWL syntax emitted for each source axiom.
    #[must_use]
    pub fn source_axioms(&self) -> &HashMap<AxiomId, OwlAxiom> {
        &self.source_axioms
    }

    /// Number of relation-record domain and range constraints rendered.
    #[must_use]
    pub fn domain_range_count(&self) -> usize {
        self.domain_range_coverage
    }
}

/// Standards OWL syntax emitted for one source axiom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwlAxiom {
    /// The axiom's OWL 2 Functional Syntax.
    pub functional_syntax: String,
}

/// A standards-level failure while translating Know IR to OWL Functional Syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwlTranslationError {
    /// The input uses a construct outside the supported OWL translation slice.
    Unsupported(String),
    /// An n-ary OWL construct lacks the operands required by its grammar.
    InvalidStructure(String),
    /// The completed Functional Syntax document could not be parsed.
    InvalidOutput(String),
}

impl Display for OwlTranslationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Unsupported(message) => write!(formatter, "unsupported OWL translation input: {message}"),
            Self::InvalidStructure(message) => write!(formatter, "invalid OWL translation structure: {message}"),
            Self::InvalidOutput(message) => write!(formatter, "invalid OWL Functional Syntax output: {message}"),
        }
    }
}

impl Error for OwlTranslationError {}

/// Translate the supported Know slice into parser-validated OWL 2 Functional Syntax.
///
/// # Errors
///
/// Returns an [`OwlTranslationError`] when the input cannot be represented as
/// valid OWL Functional Syntax.
pub fn translate_owl(module: &KnowledgeModule) -> Result<OwlTranslation, OwlTranslationError> {
    let concepts = module.concepts.iter().map(|concept| concept.id.clone()).collect::<HashSet<_>>();
    let entities = module.entities.iter().map(|entity| entity.id.clone()).collect::<HashSet<_>>();
    let relations = module.relations.iter().map(|relation| relation.id.clone()).collect::<HashSet<_>>();
    let mut lines = vec!["Ontology(".to_string()];
    let mut domain_range_coverage = 0;

    for concept in &module.concepts {
        lines.push(format!("Declaration(Class(<{}>))", concept_iri(&concept.id)));
        if let Some(definition) = &concept.definition {
            validate_expr(definition, &concepts, &relations)?;
            lines.push(format!("EquivalentClasses(<{}> {})", concept_iri(&concept.id), render_expr(definition)));
        }
    }
    for relation in &module.relations {
        lines.push(format!("Declaration(ObjectProperty(<{}>))", relation_iri(&relation.id)));
        for (kind, expression) in [("domain", &relation.domain), ("range", &relation.range)] {
            if let Some(expression) = expression {
                validate_expr(expression, &concepts, &relations)?;
                lines.push(match kind {
                    "domain" => {
                        format!("ObjectPropertyDomain(<{}> {})", relation_iri(&relation.id), render_expr(expression))
                    }
                    "range" => {
                        format!("ObjectPropertyRange(<{}> {})", relation_iri(&relation.id), render_expr(expression))
                    }
                    _ => unreachable!("domain and range are the only relation constraints"),
                });
                domain_range_coverage += 1;
            }
        }
    }
    for entity in &module.entities {
        lines.push(format!("Declaration(NamedIndividual(<{}>))", entity_iri(&entity.id)));
    }

    let mut source_axioms = HashMap::with_capacity(module.axioms.len());
    for annotated in &module.axioms {
        validate_axiom(&annotated.axiom, &concepts, &entities, &relations)?;
        let functional_syntax = render_axiom(&annotated.axiom);
        if source_axioms
            .insert(annotated.id.clone(), OwlAxiom { functional_syntax: functional_syntax.clone() })
            .is_some()
        {
            return Err(OwlTranslationError::InvalidStructure(format!("duplicate source axiom ID {}", annotated.id.0)));
        }
        lines.push(functional_syntax);
    }
    lines.push(")".to_string());
    let functional_syntax = lines.join("\n");
    validate_standard_output(&functional_syntax)?;

    Ok(OwlTranslation { functional_syntax, source_axioms, domain_range_coverage })
}

fn validate_axiom(
    axiom: &Axiom,
    concepts: &HashSet<ConceptId>,
    entities: &HashSet<EntityId>,
    relations: &HashSet<RelationId>,
) -> Result<(), OwlTranslationError> {
    match axiom {
        Axiom::SubclassOf { child, parent } => {
            validate_expr(child, concepts, relations)?;
            validate_expr(parent, concepts, relations)
        }
        Axiom::EquivalentClasses { classes } | Axiom::DisjointClasses { classes } => {
            if classes.len() < 2 {
                let kind = match axiom {
                    Axiom::EquivalentClasses { .. } => "EquivalentClasses",
                    Axiom::DisjointClasses { .. } => "DisjointClasses",
                    _ => unreachable!("only n-ary class axioms reach this branch"),
                };
                return Err(OwlTranslationError::InvalidStructure(format!(
                    "{kind} requires at least two class expressions"
                )));
            }
            for class in classes {
                validate_expr(class, concepts, relations)?;
            }
            Ok(())
        }
        Axiom::ClassAssertion { entity, class } | Axiom::NegativeClassAssertion { entity, class } => {
            require_entity(entity, entities)?;
            validate_expr(class, concepts, relations)
        }
        Axiom::RelationAssertion { subject, relation, object }
        | Axiom::NegativeRelationAssertion { subject, relation, object } => {
            require_entity(subject, entities)?;
            require_entity(object, entities)?;
            require_relation(relation, relations)
        }
    }
}

fn validate_expr(
    expr: &ConceptExpr,
    concepts: &HashSet<ConceptId>,
    relations: &HashSet<RelationId>,
) -> Result<(), OwlTranslationError> {
    match expr {
        ConceptExpr::Named(concept) => require_concept(concept, concepts),
        ConceptExpr::And(parts) | ConceptExpr::Or(parts) => {
            for part in parts {
                validate_expr(part, concepts, relations)?;
            }
            Ok(())
        }
        ConceptExpr::Not(inner) => validate_expr(inner, concepts, relations),
        ConceptExpr::Exists { relation, filler } | ConceptExpr::ForAll { relation, filler } => {
            require_relation(relation, relations)?;
            validate_expr(filler, concepts, relations)
        }
    }
}

fn require_concept(id: &ConceptId, concepts: &HashSet<ConceptId>) -> Result<(), OwlTranslationError> {
    concepts
        .contains(id)
        .then_some(())
        .ok_or_else(|| OwlTranslationError::Unsupported(format!("unknown concept {}", id.0)))
}

fn require_entity(id: &EntityId, entities: &HashSet<EntityId>) -> Result<(), OwlTranslationError> {
    entities
        .contains(id)
        .then_some(())
        .ok_or_else(|| OwlTranslationError::Unsupported(format!("unknown entity {}", id.0)))
}

fn require_relation(id: &RelationId, relations: &HashSet<RelationId>) -> Result<(), OwlTranslationError> {
    relations
        .contains(id)
        .then_some(())
        .ok_or_else(|| OwlTranslationError::Unsupported(format!("unknown relation {}", id.0)))
}

fn render_axiom(axiom: &Axiom) -> String {
    match axiom {
        Axiom::SubclassOf { child, parent } => format!("SubClassOf({} {})", render_expr(child), render_expr(parent)),
        Axiom::EquivalentClasses { classes } => format!("EquivalentClasses({})", join_exprs(classes)),
        Axiom::DisjointClasses { classes } => format!("DisjointClasses({})", join_exprs(classes)),
        Axiom::ClassAssertion { entity, class } => {
            format!("ClassAssertion({} <{}>)", render_expr(class), entity_iri(entity))
        }
        Axiom::NegativeClassAssertion { entity, class } => {
            format!("ClassAssertion(ObjectComplementOf({}) <{}>)", render_expr(class), entity_iri(entity))
        }
        Axiom::RelationAssertion { subject, relation, object } => {
            format!(
                "ObjectPropertyAssertion(<{}> <{}> <{}>)",
                relation_iri(relation),
                entity_iri(subject),
                entity_iri(object)
            )
        }
        Axiom::NegativeRelationAssertion { subject, relation, object } => {
            format!(
                "NegativeObjectPropertyAssertion(<{}> <{}> <{}>)",
                relation_iri(relation),
                entity_iri(subject),
                entity_iri(object)
            )
        }
    }
}

pub(crate) fn render_expr(expr: &ConceptExpr) -> String {
    match expr {
        ConceptExpr::Named(id) => format!("<{}>", concept_iri(id)),
        ConceptExpr::And(parts) => render_boolean("ObjectIntersectionOf", OWL_THING, parts),
        ConceptExpr::Or(parts) => render_boolean("ObjectUnionOf", OWL_NOTHING, parts),
        ConceptExpr::Not(inner) => format!("ObjectComplementOf({})", render_expr(inner)),
        ConceptExpr::Exists { relation, filler } => {
            format!("ObjectSomeValuesFrom(<{}> {})", relation_iri(relation), render_expr(filler))
        }
        ConceptExpr::ForAll { relation, filler } => {
            format!("ObjectAllValuesFrom(<{}> {})", relation_iri(relation), render_expr(filler))
        }
    }
}

fn join_exprs(expressions: &[ConceptExpr]) -> String {
    expressions.iter().map(render_expr).collect::<Vec<_>>().join(" ")
}

fn render_boolean(operator: &str, identity: &str, expressions: &[ConceptExpr]) -> String {
    match expressions {
        [] => format!("<{identity}>"),
        [expression] => render_expr(expression),
        _ => format!("{operator}({})", join_exprs(expressions)),
    }
}

fn validate_standard_output(owl: &str) -> Result<(), OwlTranslationError> {
    let mut reader = Cursor::new(owl);
    read_ofn::<RcStr, SetOntology<RcStr>, _>(&mut reader, ParserConfiguration::default())
        .map(|_| ())
        .map_err(|error| OwlTranslationError::InvalidOutput(error.to_string()))
}
