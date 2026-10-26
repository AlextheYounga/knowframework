use know_ontology::{Axiom, ConceptExpr, KnowledgeModule};

use crate::iri::{OWL_NOTHING, OWL_THING, concept_iri, entity_iri, relation_iri};

pub(crate) fn adapt(module: &KnowledgeModule) -> String {
    let mut lines = vec!["Ontology(".to_string()];
    for concept in &module.concepts {
        lines.push(format!("Declaration(Class(<{}>))", concept_iri(&concept.id)));
        if let Some(definition) = &concept.definition {
            lines.push(format!("EquivalentClasses(<{}> {})", concept_iri(&concept.id), render_expr(definition)));
        }
    }
    for relation in &module.relations {
        lines.push(format!("Declaration(ObjectProperty(<{}>))", relation_iri(&relation.id)));
        if let Some(domain) = &relation.domain {
            lines.push(format!("ObjectPropertyDomain(<{}> {})", relation_iri(&relation.id), render_expr(domain)));
        }
        if let Some(range) = &relation.range {
            lines.push(format!("ObjectPropertyRange(<{}> {})", relation_iri(&relation.id), render_expr(range)));
        }
    }
    for entity in &module.entities {
        lines.push(format!("Declaration(NamedIndividual(<{}>))", entity_iri(&entity.id)));
    }
    lines.extend(module.axioms.iter().map(|annotated| render_axiom(&annotated.axiom)));
    lines.push(")".to_string());
    lines.join("\n")
}

pub(crate) fn render_expr(expr: &ConceptExpr) -> String {
    render_expr_nnf(expr, false)
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
            format!("ClassAssertion({} <{}>)", render_expr_nnf(class, true), entity_iri(entity))
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

fn render_expr_nnf(expr: &ConceptExpr, negated: bool) -> String {
    match (expr, negated) {
        (ConceptExpr::Named(id), false) => format!("<{}>", concept_iri(id)),
        (ConceptExpr::Named(id), true) => format!("ObjectComplementOf(<{}>)", concept_iri(id)),
        (ConceptExpr::And(parts), false) => {
            render_boolean("ObjectIntersectionOf", OWL_THING, parts, false)
        }
        (ConceptExpr::And(parts), true) => {
            render_boolean("ObjectUnionOf", OWL_NOTHING, parts, true)
        }
        (ConceptExpr::Or(parts), false) => {
            render_boolean("ObjectUnionOf", OWL_NOTHING, parts, false)
        }
        (ConceptExpr::Or(parts), true) => {
            render_boolean("ObjectIntersectionOf", OWL_THING, parts, true)
        }
        (ConceptExpr::Not(inner), negated) => render_expr_nnf(inner, !negated),
        (ConceptExpr::Exists { relation, filler }, false) => {
            format!("ObjectSomeValuesFrom(<{}> {})", relation_iri(relation), render_expr_nnf(filler, false))
        }
        (ConceptExpr::Exists { relation, filler }, true) => {
            format!("ObjectAllValuesFrom(<{}> {})", relation_iri(relation), render_expr_nnf(filler, true))
        }
        (ConceptExpr::ForAll { relation, filler }, false) => {
            format!("ObjectAllValuesFrom(<{}> {})", relation_iri(relation), render_expr_nnf(filler, false))
        }
        (ConceptExpr::ForAll { relation, filler }, true) => {
            format!("ObjectSomeValuesFrom(<{}> {})", relation_iri(relation), render_expr_nnf(filler, true))
        }
    }
}

fn join_exprs(expressions: &[ConceptExpr]) -> String {
    expressions.iter().map(render_expr).collect::<Vec<_>>().join(" ")
}

fn render_boolean(operator: &str, identity: &str, expressions: &[ConceptExpr], negated: bool) -> String {
    match expressions {
        [] => format!("<{identity}>"),
        [expression] => render_expr_nnf(expression, negated),
        _ => {
            let parts =
                expressions.iter().map(|expression| render_expr_nnf(expression, negated)).collect::<Vec<_>>().join(" ");
            format!("{operator}({parts})")
        }
    }
}
