//! OWL 2 Functional Syntax conversion and the in-process rustdl backend.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

use horned_owl::io::ParserConfiguration;
use horned_owl::io::ofn::reader::read as read_ofn;
use horned_owl::model::RcStr;
use horned_owl::ontology::set::SetOntology;
use know_core::{AxiomId, ConceptId, EntityId, RelationId};
use know_ontology::{ConceptExpr, KnowledgeModule};
use know_reasoner::{Proposition, ReasoningOutcome};

mod backend;
mod init;
mod iri;
mod outcome;
mod query;
mod translate;

pub use init::RustdlInitError;
pub use iri::{
    IriCodecError, concept_iri, decode_concept_iri, decode_entity_iri, decode_relation_iri, entity_iri, relation_iri,
};
pub use translate::{OwlAxiom, OwlTranslation, OwlTranslationError, translate_owl};

/// A rustdl-backed reasoner for Know's first OWL-aligned object-property slice.
/// The canonical representation remains [`KnowledgeModule`].
pub struct RustdlReasoner {
    owl: String,
    concepts: HashSet<ConceptId>,
    entities: HashSet<EntityId>,
    relations: HashSet<RelationId>,
    source_axioms: HashMap<AxiomId, OwlAxiom>,
    domain_range_coverage: usize,
}

impl RustdlReasoner {
    /// Translate and validate the complete module before accepting any query.
    ///
    /// # Errors
    ///
    /// Returns [`RustdlInitError::Unsupported`] for input outside the adapter
    /// slice and [`RustdlInitError::InternalError`] for parser or backend setup failures.
    pub fn new(module: &KnowledgeModule) -> Result<Self, RustdlInitError> {
        let translated = translate_owl(module).map_err(map_translation_error)?;
        let backend_ontology = backend::adapt(module);
        initialize_backend(&backend_ontology)?;

        Ok(Self {
            owl: backend_ontology,
            concepts: module.concepts.iter().map(|concept| concept.id.clone()).collect(),
            entities: module.entities.iter().map(|entity| entity.id.clone()).collect(),
            relations: module.relations.iter().map(|relation| relation.id.clone()).collect(),
            source_axioms: translated.source_axioms().clone(),
            domain_range_coverage: translated.domain_range_count(),
        })
    }

    /// Source axiom IDs represented in the immutable translated ontology.
    #[must_use]
    pub fn translated_axioms(&self) -> &HashMap<AxiomId, OwlAxiom> {
        &self.source_axioms
    }

    /// Number of relation-record domain and range constraints represented.
    #[must_use]
    pub fn translated_domain_range_count(&self) -> usize {
        self.domain_range_coverage
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

    fn validate_proposition(&self, proposition: &Proposition) -> Option<String> {
        match proposition {
            Proposition::ClassMembership { entity, class } => self
                .entities
                .contains(entity)
                .then_some(())
                .and_then(|()| self.validate_expr(class))
                .or_else(|| (!self.entities.contains(entity)).then(|| format!("unknown entity {}", entity.0))),
            Proposition::SubclassOf { child, parent }
            | Proposition::Equivalent { left: child, right: parent }
            | Proposition::Disjoint { left: child, right: parent } => {
                self.validate_expr(child).or_else(|| self.validate_expr(parent))
            }
            Proposition::Satisfiable { class } => self.validate_expr(class),
            Proposition::RelationHolds { subject, relation, object } => (!self.entities.contains(subject))
                .then(|| format!("unknown entity {}", subject.0))
                .or_else(|| (!self.entities.contains(object)).then(|| format!("unknown entity {}", object.0)))
                .or_else(|| (!self.relations.contains(relation)).then(|| format!("unknown relation {}", relation.0))),
            Proposition::Consistent => None,
        }
    }

    fn validate_expr(&self, expr: &ConceptExpr) -> Option<String> {
        match expr {
            ConceptExpr::Named(id) => (!self.concepts.contains(id)).then(|| format!("unknown concept {}", id.0)),
            ConceptExpr::And(parts) | ConceptExpr::Or(parts) => parts.iter().find_map(|part| self.validate_expr(part)),
            ConceptExpr::Not(inner) => self.validate_expr(inner),
            ConceptExpr::Exists { relation, filler } | ConceptExpr::ForAll { relation, filler } => {
                (!self.relations.contains(relation))
                    .then(|| format!("unknown relation {}", relation.0))
                    .or_else(|| self.validate_expr(filler))
            }
        }
    }

    fn consistency(&self) -> ReasoningOutcome<bool> {
        self.run_ontology(&[], owl_dl_reasoner::is_consistent)
    }

    fn satisfiable(&self, expr: &ConceptExpr) -> ReasoningOutcome<bool> {
        let probe = "urn:know:probe:satisfiable";
        let additions = [
            format!("Declaration(Class(<{probe}>))"),
            format!("EquivalentClasses(<{probe}> {})", backend::render_expr(expr)),
        ];
        self.run_ontology(&additions, |ontology| owl_dl_reasoner::is_class_satisfiable(ontology, probe))
    }

    fn instance_of(&self, entity: &EntityId, expr: &ConceptExpr) -> ReasoningOutcome<bool> {
        let probe = "urn:know:probe:instance";
        let additions = [
            format!("Declaration(Class(<{probe}>))"),
            format!("EquivalentClasses(<{probe}> {})", backend::render_expr(expr)),
        ];
        self.run_ontology(&additions, |ontology| owl_dl_reasoner::is_instance_of(ontology, probe, &entity_iri(entity)))
    }

    fn subclass_of(&self, child: &ConceptExpr, parent: &ConceptExpr) -> ReasoningOutcome<bool> {
        let child_probe = "urn:know:probe:subclass-child";
        let parent_probe = "urn:know:probe:subclass-parent";
        let additions = [
            format!("Declaration(Class(<{child_probe}>))"),
            format!("Declaration(Class(<{parent_probe}>))"),
            format!("EquivalentClasses(<{child_probe}> {})", backend::render_expr(child)),
            format!("EquivalentClasses(<{parent_probe}> {})", backend::render_expr(parent)),
        ];
        self.run_ontology(&additions, |ontology| owl_dl_reasoner::is_subclass_of(ontology, child_probe, parent_probe))
    }

    fn consistent_with(&self, addition: String) -> ReasoningOutcome<bool> {
        self.run_ontology(&[addition], owl_dl_reasoner::is_consistent)
    }

    fn run_ontology(
        &self,
        additions: &[String],
        query: impl FnOnce(&SetOntology<RcStr>) -> Result<bool, owl_dl_reasoner::ReasonError>,
    ) -> ReasoningOutcome<bool> {
        let ontology = match self.query_ontology(additions) {
            Ok(ontology) => ontology,
            Err(error) => return ReasoningOutcome::InternalError(error),
        };
        match catch_unwind(AssertUnwindSafe(|| query(&ontology))) {
            Ok(result) => outcome::normalize(result),
            Err(_) => ReasoningOutcome::InternalError("rustdl panicked while evaluating the query".to_string()),
        }
    }
}

/// Export the accepted Know fragment as OWL 2 Functional Syntax.
///
/// # Errors
///
/// Returns [`OwlTranslationError`] when the module cannot be represented as valid
/// OWL 2 Functional Syntax.
pub fn export_owl_functional(module: &KnowledgeModule) -> Result<String, OwlTranslationError> {
    translate_owl(module).map(|translation| translation.functional_syntax().to_string())
}

fn map_translation_error(error: OwlTranslationError) -> RustdlInitError {
    match error {
        OwlTranslationError::Unsupported(message) | OwlTranslationError::InvalidStructure(message) => {
            RustdlInitError::Unsupported(message)
        }
        OwlTranslationError::InvalidOutput(message) => RustdlInitError::InternalError(message),
    }
}

/// OWL import remains deferred until Know's supported import subset is defined.
///
/// # Errors
///
/// Always returns an error because OWL import is outside this adapter's scope.
pub fn import_owl_functional(_owl_source: &str) -> Result<know_ontology::KnowledgeModuleSource, String> {
    Err("OWL import not yet implemented".to_string())
}

fn parse_ontology(source: &str) -> Result<SetOntology<RcStr>, String> {
    let mut reader = Cursor::new(source);
    read_ofn(&mut reader, ParserConfiguration::default())
        .map(|(ontology, _)| ontology)
        .map_err(|error| error.to_string())
}

fn initialize_backend(source: &str) -> Result<(), RustdlInitError> {
    let ontology = parse_ontology(source).map_err(RustdlInitError::InternalError)?;
    let initialization =
        catch_unwind(AssertUnwindSafe(|| owl_dl_reasoner::is_consistent(&ontology))).map_err(|_| {
            RustdlInitError::InternalError("rustdl panicked while initializing the translated ontology".to_string())
        })?;
    initialization.map(|_| ()).map_err(outcome::init_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_backend_syntax_maps_to_initialization_internal_error() {
        assert!(matches!(
            initialize_backend("Ontology("),
            Err(RustdlInitError::InternalError(message)) if !message.is_empty()
        ));
    }

    #[test]
    fn translation_errors_map_to_the_correct_initialization_boundary() {
        assert!(matches!(
            map_translation_error(OwlTranslationError::Unsupported("unsupported".to_string())),
            RustdlInitError::Unsupported(_)
        ));
        assert!(matches!(
            map_translation_error(OwlTranslationError::InvalidStructure("invalid".to_string())),
            RustdlInitError::Unsupported(_)
        ));
        assert!(matches!(
            map_translation_error(OwlTranslationError::InvalidOutput("broken output".to_string())),
            RustdlInitError::InternalError(_)
        ));
    }
}
