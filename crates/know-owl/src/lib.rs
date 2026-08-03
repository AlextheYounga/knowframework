//! OWL 2 Functional Syntax conversion and the in-process rustdl backend.

use std::io::Cursor;

use horned_owl::io::ParserConfiguration;
use horned_owl::io::ofn::reader::read as read_ofn;
use horned_owl::model::RcStr;
use horned_owl::ontology::set::SetOntology;
use know_core::{AxiomId, ConceptId, EntityId, RelationId};
use know_ontology::{Axiom, ConceptExpr, KnowledgeModule};
use know_reasoner::{
    ClassificationResult, Explanation, InconsistencyReport, InferenceRule, InferenceStep, Proposition, Reasoner,
    ReasoningOutcome, ResourceLimit, ResourceLimitKind, UnknownExplanation, UnsupportedFeature, Verdict,
};

/// A rustdl-backed reasoner for Know's current OWL-aligned object-property
/// fragment. The canonical representation remains `KnowledgeModule`.
pub struct RustdlReasoner {
    owl: String,
    concepts: Vec<ConceptId>,
    entities: Vec<EntityId>,
    relations: Vec<RelationId>,
}

impl RustdlReasoner {
    pub fn new(module: &KnowledgeModule) -> Result<Self, String> {
        let owl = export_owl_functional(module)?;
        parse_ontology(&owl)?;
        Ok(Self {
            owl,
            concepts: module.concepts.iter().map(|concept| concept.id.clone()).collect(),
            entities: module.entities.iter().map(|entity| entity.id.clone()).collect(),
            relations: module.relations.iter().map(|relation| relation.id.clone()).collect(),
        })
    }

    fn query_ontology(&self, additions: &[String]) -> Result<SetOntology<RcStr>, String> {
        let mut source = self.owl.trim_end().trim_end_matches(')').trim_end().to_string();
        for addition in additions {
            source.push('\n');
            source.push_str(addition);
        }
        source.push_str("\n)\n");
        parse_ontology(&source)
    }

    fn has_concept(&self, id: &ConceptId) -> bool {
        self.concepts.contains(id)
    }

    fn has_entity(&self, id: &EntityId) -> bool {
        self.entities.contains(id)
    }

    fn has_relation(&self, id: &RelationId) -> bool {
        self.relations.contains(id)
    }

    fn validate_expr(&self, expr: &ConceptExpr) -> Option<String> {
        match expr {
            ConceptExpr::Named(id) if !self.has_concept(id) => Some(format!("unknown concept {}", id.0)),
            ConceptExpr::Named(_) => None,
            ConceptExpr::And(parts) | ConceptExpr::Or(parts) => parts.iter().find_map(|part| self.validate_expr(part)),
            ConceptExpr::Not(inner) => self.validate_expr(inner),
            ConceptExpr::Exists { relation, filler } | ConceptExpr::ForAll { relation, filler } => {
                if !self.has_relation(relation) {
                    Some(format!("unknown relation {}", relation.0))
                } else {
                    self.validate_expr(filler)
                }
            }
        }
    }

    fn satisfiable(&self, expr: &ConceptExpr) -> Result<bool, String> {
        let probe = "urn:know:probe:satisfiable";
        let ontology = self.query_ontology(&[
            format!("Declaration(Class(<{probe}>))"),
            format!("EquivalentClasses(<{probe}> {})", render_expr(expr)),
        ])?;
        owl_dl_reasoner::is_class_satisfiable(&ontology, probe).map_err(|error| error.to_string())
    }

    fn instance_of(&self, entity: &EntityId, expr: &ConceptExpr) -> Result<bool, String> {
        let probe = "urn:know:probe:instance";
        let ontology = self.query_ontology(&[
            format!("Declaration(Class(<{probe}>))"),
            format!("EquivalentClasses(<{probe}> {})", render_expr(expr)),
        ])?;
        owl_dl_reasoner::is_instance_of(&ontology, probe, &entity_iri(entity)).map_err(|error| error.to_string())
    }

    fn entails_subclass(&self, child: &ConceptExpr, parent: &ConceptExpr) -> Result<bool, String> {
        let witness = "urn:know:probe:subclass-witness";
        let ontology = self.query_ontology(&[
            format!("Declaration(NamedIndividual(<{witness}>))"),
            format!(
                "ClassAssertion(ObjectIntersectionOf({} ObjectComplementOf({})) <{witness}>)",
                render_expr(child),
                render_expr(parent),
            ),
        ])?;
        owl_dl_reasoner::is_consistent(&ontology).map(|consistent| !consistent).map_err(|error| error.to_string())
    }

    fn consistent_with(&self, addition: String) -> Result<bool, String> {
        let ontology = self.query_ontology(&[addition])?;
        owl_dl_reasoner::is_consistent(&ontology).map_err(|error| error.to_string())
    }

    fn explanation(proposition: &Proposition, note: &str) -> Explanation {
        Explanation {
            conclusion: proposition.clone(),
            supporting_axioms: Vec::<AxiomId>::new(),
            steps: vec![InferenceStep {
                rule: InferenceRule::BooleanRefutation,
                premises: vec![proposition.clone()],
                conclusion: proposition.clone(),
            }],
            notes: Some(note.to_string()),
        }
    }

    fn unsupported(feature: impl Into<String>) -> ReasoningOutcome<Verdict> {
        ReasoningOutcome::Unsupported(UnsupportedFeature { feature: feature.into(), advice: None })
    }

    fn backend_error(error: String) -> ReasoningOutcome<Verdict> {
        if error.contains("NoVerdict") || error.contains("without a verdict") {
            ReasoningOutcome::ResourceLimit(ResourceLimit { kind: ResourceLimitKind::Iterations })
        } else {
            Self::unsupported(format!("rustdl could not evaluate this ontology: {error}"))
        }
    }
}

impl Reasoner for RustdlReasoner {
    fn query(&self, proposition: &Proposition) -> ReasoningOutcome<Verdict> {
        let invalid = match proposition {
            Proposition::ClassMembership { entity, class } => (!self.has_entity(entity))
                .then(|| format!("unknown entity {}", entity.0))
                .or_else(|| self.validate_expr(class)),
            Proposition::SubclassOf { child, parent }
            | Proposition::Equivalent { left: child, right: parent }
            | Proposition::Disjoint { left: child, right: parent } => {
                self.validate_expr(child).or_else(|| self.validate_expr(parent))
            }
            Proposition::Satisfiable { class } => self.validate_expr(class),
            Proposition::RelationHolds { subject, relation, object } => (!self.has_entity(subject))
                .then(|| format!("unknown entity {}", subject.0))
                .or_else(|| (!self.has_entity(object)).then(|| format!("unknown entity {}", object.0)))
                .or_else(|| (!self.has_relation(relation)).then(|| format!("unknown relation {}", relation.0))),
            Proposition::Consistent => None,
        };
        if let Some(error) = invalid {
            return ReasoningOutcome::Complete(Verdict::IllTyped(vec![know_core::Diagnostic::error(
                know_core::codes::UNSUPPORTED_FEATURE,
                error,
            )]));
        }

        let consistency = self.consistent_with(String::new());
        match consistency {
            Ok(false) => {
                return ReasoningOutcome::Complete(Verdict::Inconsistent(InconsistencyReport {
                    conflicting_axioms: vec![],
                    explanation: Some("rustdl found the ontology inconsistent".to_string()),
                }));
            }
            Err(error) => return Self::backend_error(error),
            Ok(true) => {}
        }

        let result = match proposition {
            Proposition::Consistent => Ok(Verdict::Entailed(Self::explanation(proposition, "rustdl found a model"))),
            Proposition::Satisfiable { class } => self.satisfiable(class).map(|satisfiable| {
                if satisfiable {
                    Verdict::Entailed(Self::explanation(proposition, "rustdl found a model for the class"))
                } else {
                    Verdict::Contradicted(Self::explanation(proposition, "rustdl proved the class unsatisfiable"))
                }
            }),
            Proposition::SubclassOf { child, parent } => self.entails_subclass(child, parent).map(|entailed| {
                if entailed {
                    Verdict::Entailed(Self::explanation(proposition, "rustdl refuted a counterexample"))
                } else {
                    Verdict::Unknown(UnknownExplanation {
                        proposition: proposition.clone(),
                        missing: vec!["subclass relation is not entailed".to_string()],
                    })
                }
            }),
            Proposition::Equivalent { left, right } => self
                .entails_subclass(left, right)
                .and_then(|forward| self.entails_subclass(right, left).map(|reverse| forward && reverse))
                .map(|entailed| {
                    if entailed {
                        Verdict::Entailed(Self::explanation(proposition, "rustdl proved both subclass directions"))
                    } else {
                        Verdict::Unknown(UnknownExplanation {
                            proposition: proposition.clone(),
                            missing: vec!["equivalence is not entailed".to_string()],
                        })
                    }
                }),
            Proposition::Disjoint { left, right } => {
                self.satisfiable(&ConceptExpr::And(vec![left.clone(), right.clone()])).map(|satisfiable| {
                    if satisfiable {
                        Verdict::Unknown(UnknownExplanation {
                            proposition: proposition.clone(),
                            missing: vec!["disjointness is not entailed".to_string()],
                        })
                    } else {
                        Verdict::Entailed(Self::explanation(
                            proposition,
                            "rustdl proved the intersection unsatisfiable",
                        ))
                    }
                })
            }
            Proposition::ClassMembership { entity, class } => self.instance_of(entity, class).and_then(|entailed| {
                if entailed {
                    Ok(Verdict::Entailed(Self::explanation(proposition, "rustdl proved class membership")))
                } else {
                    self.instance_of(entity, &ConceptExpr::Not(Box::new(class.clone()))).map(|contradicted| {
                        if contradicted {
                            Verdict::Contradicted(Self::explanation(
                                proposition,
                                "rustdl proved negative class membership",
                            ))
                        } else {
                            Verdict::Unknown(UnknownExplanation {
                                proposition: proposition.clone(),
                                missing: vec!["class membership is not entailed".to_string()],
                            })
                        }
                    })
                }
            }),
            Proposition::RelationHolds { subject, relation, object } => {
                let positive = format!(
                    "ObjectPropertyAssertion(<{}> <{}> <{}>)",
                    relation_iri(relation),
                    entity_iri(subject),
                    entity_iri(object)
                );
                let negative = format!(
                    "NegativeObjectPropertyAssertion(<{}> <{}> <{}>)",
                    relation_iri(relation),
                    entity_iri(subject),
                    entity_iri(object)
                );
                self.consistent_with(negative).and_then(|negative_consistent| {
                    if !negative_consistent {
                        Ok(Verdict::Entailed(Self::explanation(
                            proposition,
                            "rustdl refuted the negative property assertion",
                        )))
                    } else {
                        self.consistent_with(positive).map(|positive_consistent| {
                            if !positive_consistent {
                                Verdict::Contradicted(Self::explanation(
                                    proposition,
                                    "rustdl refuted the property assertion",
                                ))
                            } else {
                                Verdict::Unknown(UnknownExplanation {
                                    proposition: proposition.clone(),
                                    missing: vec!["property assertion is not entailed".to_string()],
                                })
                            }
                        })
                    }
                })
            }
        };
        match result {
            Ok(verdict) => ReasoningOutcome::Complete(verdict),
            Err(error) => Self::backend_error(error),
        }
    }

    fn classify(&self, _concept: &ConceptExpr) -> ReasoningOutcome<ClassificationResult> {
        ReasoningOutcome::Unsupported(UnsupportedFeature {
            feature: "rustdl classification is not yet mapped to Know direct hierarchy results".to_string(),
            advice: None,
        })
    }

    fn is_consistent(&self) -> ReasoningOutcome<bool> {
        match self.consistent_with(String::new()) {
            Ok(consistent) => ReasoningOutcome::Complete(consistent),
            Err(error) => match Self::backend_error(error) {
                ReasoningOutcome::Unsupported(feature) => ReasoningOutcome::Unsupported(feature),
                ReasoningOutcome::ResourceLimit(limit) => ReasoningOutcome::ResourceLimit(limit),
                _ => unreachable!(),
            },
        }
    }

    fn is_satisfiable(&self, concept: &ConceptExpr) -> ReasoningOutcome<bool> {
        if let Some(error) = self.validate_expr(concept) {
            return ReasoningOutcome::Unsupported(UnsupportedFeature { feature: error, advice: None });
        }
        match self.satisfiable(concept) {
            Ok(satisfiable) => ReasoningOutcome::Complete(satisfiable),
            Err(error) => match Self::backend_error(error) {
                ReasoningOutcome::Unsupported(feature) => ReasoningOutcome::Unsupported(feature),
                ReasoningOutcome::ResourceLimit(limit) => ReasoningOutcome::ResourceLimit(limit),
                _ => unreachable!(),
            },
        }
    }
}

/// Export the represented Know fragment as OWL 2 Functional Syntax.
pub fn export_owl_functional(module: &KnowledgeModule) -> Result<String, String> {
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
    for annotated in &module.axioms {
        lines.push(render_axiom(&annotated.axiom));
    }
    lines.push(")".to_string());
    Ok(lines.join("\n"))
}

/// OWL import remains deferred until Know's supported import subset is defined.
pub fn import_owl_functional(_owl_source: &str) -> Result<know_ontology::KnowledgeModuleSource, String> {
    Err("OWL import not yet implemented".to_string())
}

fn parse_ontology(source: &str) -> Result<SetOntology<RcStr>, String> {
    let mut reader = Cursor::new(source);
    read_ofn(&mut reader, ParserConfiguration::default())
        .map(|(ontology, _)| ontology)
        .map_err(|error| error.to_string())
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
            format!("NegativeClassAssertion({} <{}>)", render_expr(class), entity_iri(entity))
        }
        Axiom::RelationAssertion { subject, relation, object } => format!(
            "ObjectPropertyAssertion(<{}> <{}> <{}>)",
            relation_iri(relation),
            entity_iri(subject),
            entity_iri(object)
        ),
        Axiom::NegativeRelationAssertion { subject, relation, object } => format!(
            "NegativeObjectPropertyAssertion(<{}> <{}> <{}>)",
            relation_iri(relation),
            entity_iri(subject),
            entity_iri(object)
        ),
    }
}

fn render_expr(expr: &ConceptExpr) -> String {
    match expr {
        ConceptExpr::Named(id) => format!("<{}>", concept_iri(id)),
        ConceptExpr::And(parts) => format!("ObjectIntersectionOf({})", join_exprs(parts)),
        ConceptExpr::Or(parts) => format!("ObjectUnionOf({})", join_exprs(parts)),
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

fn concept_iri(id: &ConceptId) -> String {
    iri("concept", &id.0)
}

fn relation_iri(id: &RelationId) -> String {
    iri("relation", &id.0)
}

fn entity_iri(id: &EntityId) -> String {
    iri("entity", &id.0)
}

fn iri(kind: &str, id: &str) -> String {
    let encoded = id.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    format!("urn:know:{kind}:{encoded}")
}
