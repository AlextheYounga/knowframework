use know_core::{EntityId, codes};
use know_ontology::ConceptExpr;
use know_reasoner::{
    ClassificationResult, Explanation, InconsistencyReport, Proposition, Reasoner, ReasoningOutcome,
    UnknownExplanation, UnsupportedFeature, Verdict,
};

use crate::{RustdlReasoner, backend::render_expr, entity_iri, outcome, relation_iri};

impl RustdlReasoner {
    fn explanation(proposition: &Proposition, note: &str) -> Explanation {
        Explanation {
            conclusion: proposition.clone(),
            supporting_axioms: Vec::new(),
            steps: Vec::new(),
            notes: Some(note.to_string()),
        }
    }

    fn unknown(proposition: &Proposition, missing: &str) -> Verdict {
        Verdict::Unknown(UnknownExplanation { proposition: proposition.clone(), missing: vec![missing.to_string()] })
    }

    fn subclass_verdict(
        &self,
        proposition: &Proposition,
        child: &ConceptExpr,
        parent: &ConceptExpr,
    ) -> ReasoningOutcome<Verdict> {
        match self.subclass_of(child, parent) {
            ReasoningOutcome::Complete(true) => ReasoningOutcome::Complete(Verdict::Entailed(Self::explanation(
                proposition,
                "rustdl completed the subclass check",
            ))),
            ReasoningOutcome::Complete(false) => {
                let constraint = format!("SubClassOf({} {})", render_expr(child), render_expr(parent));
                match self.consistent_with(constraint) {
                    ReasoningOutcome::Complete(false) => ReasoningOutcome::Complete(Verdict::Contradicted(
                        Self::explanation(proposition, "rustdl completed the subclass negation consistency check"),
                    )),
                    ReasoningOutcome::Complete(true) => ReasoningOutcome::Complete(Self::unknown(
                        proposition,
                        "the subclass relation and its negation both have a model",
                    )),
                    outcome => outcome::propagate(outcome),
                }
            }
            outcome => outcome::propagate(outcome),
        }
    }

    fn equivalence_verdict(
        &self,
        proposition: &Proposition,
        left: &ConceptExpr,
        right: &ConceptExpr,
    ) -> ReasoningOutcome<Verdict> {
        let forward = match self.subclass_of(left, right) {
            ReasoningOutcome::Complete(value) => value,
            outcome => return outcome::propagate(outcome),
        };
        let backward = match self.subclass_of(right, left) {
            ReasoningOutcome::Complete(value) => value,
            outcome => return outcome::propagate(outcome),
        };
        if forward && backward {
            return ReasoningOutcome::Complete(Verdict::Entailed(Self::explanation(
                proposition,
                "rustdl completed both subclass checks for equivalence",
            )));
        }

        let constraint = format!("EquivalentClasses({} {})", render_expr(left), render_expr(right));
        match self.consistent_with(constraint) {
            ReasoningOutcome::Complete(false) => ReasoningOutcome::Complete(Verdict::Contradicted(Self::explanation(
                proposition,
                "rustdl completed the equivalence negation consistency check",
            ))),
            ReasoningOutcome::Complete(true) => ReasoningOutcome::Complete(Self::unknown(
                proposition,
                "the equivalence relation and its negation both have a model",
            )),
            outcome => outcome::propagate(outcome),
        }
    }
}

impl Reasoner for RustdlReasoner {
    fn query(&self, proposition: &Proposition) -> ReasoningOutcome<Verdict> {
        if let Some(error) = self.validate_proposition(proposition) {
            return ReasoningOutcome::Complete(Verdict::IllTyped(vec![know_core::Diagnostic::error(
                codes::UNSUPPORTED_FEATURE,
                error,
            )]));
        }
        match self.consistency() {
            ReasoningOutcome::Complete(false) => {
                return ReasoningOutcome::Complete(Verdict::Inconsistent(InconsistencyReport {
                    conflicting_axioms: Vec::new(),
                    explanation: Some(
                        "rustdl completed an inconsistency check without a mapped justification".to_string(),
                    ),
                }));
            }
            ReasoningOutcome::Complete(true) => {}
            outcome => return outcome::propagate(outcome),
        }

        match proposition {
            Proposition::Consistent => ReasoningOutcome::Complete(Verdict::Entailed(Self::explanation(
                proposition,
                "rustdl completed the consistency check",
            ))),
            Proposition::Satisfiable { class } => verdict_from_satisfiability(self.satisfiable(class), proposition),
            Proposition::SubclassOf { child, parent } => self.subclass_verdict(proposition, child, parent),
            Proposition::Equivalent { left, right } => self.equivalence_verdict(proposition, left, right),
            Proposition::Disjoint { left, right } => {
                match self.satisfiable(&ConceptExpr::And(vec![left.clone(), right.clone()])) {
                    ReasoningOutcome::Complete(false) => ReasoningOutcome::Complete(Verdict::Entailed(
                        Self::explanation(proposition, "rustdl completed the intersection unsatisfiability check"),
                    )),
                    ReasoningOutcome::Complete(true) => {
                        let constraint = format!("DisjointClasses({} {})", render_expr(left), render_expr(right));
                        match self.consistent_with(constraint) {
                            ReasoningOutcome::Complete(false) => {
                                ReasoningOutcome::Complete(Verdict::Contradicted(Self::explanation(
                                    proposition,
                                    "rustdl completed the disjointness negation consistency check",
                                )))
                            }
                            ReasoningOutcome::Complete(true) => ReasoningOutcome::Complete(Self::unknown(
                                proposition,
                                "the intersection and disjointness constraint both have a model",
                            )),
                            outcome => outcome::propagate(outcome),
                        }
                    }
                    outcome => outcome::propagate(outcome),
                }
            }
            Proposition::ClassMembership { entity, class } => membership_verdict(self, proposition, entity, class),
            Proposition::RelationHolds { .. } => relation_verdict(self, proposition),
        }
    }

    fn classify(&self, _concept: &ConceptExpr) -> ReasoningOutcome<ClassificationResult> {
        ReasoningOutcome::Unsupported(UnsupportedFeature {
            feature: "rustdl classification is not mapped to Know direct hierarchy results".to_string(),
            advice: None,
        })
    }

    fn is_consistent(&self) -> ReasoningOutcome<bool> {
        self.consistency()
    }

    fn is_satisfiable(&self, concept: &ConceptExpr) -> ReasoningOutcome<bool> {
        if let Some(error) = self.validate_expr(concept) {
            return ReasoningOutcome::Unsupported(UnsupportedFeature { feature: error, advice: None });
        }
        self.satisfiable(concept)
    }
}

fn verdict_from_satisfiability(
    outcome: ReasoningOutcome<bool>,
    proposition: &Proposition,
) -> ReasoningOutcome<Verdict> {
    match outcome {
        ReasoningOutcome::Complete(true) => ReasoningOutcome::Complete(Verdict::Entailed(RustdlReasoner::explanation(
            proposition,
            "rustdl completed the satisfiability check",
        ))),
        ReasoningOutcome::Complete(false) => ReasoningOutcome::Complete(Verdict::Contradicted(
            RustdlReasoner::explanation(proposition, "rustdl completed the unsatisfiability check"),
        )),
        outcome => outcome::propagate(outcome),
    }
}

fn membership_verdict(
    reasoner: &RustdlReasoner,
    proposition: &Proposition,
    entity: &EntityId,
    class: &ConceptExpr,
) -> ReasoningOutcome<Verdict> {
    match reasoner.instance_of(entity, class) {
        ReasoningOutcome::Complete(true) => ReasoningOutcome::Complete(Verdict::Entailed(RustdlReasoner::explanation(
            proposition,
            "rustdl completed the positive membership check",
        ))),
        ReasoningOutcome::Complete(false) => {
            match reasoner.instance_of(entity, &ConceptExpr::Not(Box::new(class.clone()))) {
                ReasoningOutcome::Complete(true) => ReasoningOutcome::Complete(Verdict::Contradicted(
                    RustdlReasoner::explanation(proposition, "rustdl completed the negative membership check"),
                )),
                ReasoningOutcome::Complete(false) => ReasoningOutcome::Complete(RustdlReasoner::unknown(
                    proposition,
                    "neither class membership nor its complement is entailed",
                )),
                outcome => outcome::propagate(outcome),
            }
        }
        outcome => outcome::propagate(outcome),
    }
}

fn relation_verdict(reasoner: &RustdlReasoner, proposition: &Proposition) -> ReasoningOutcome<Verdict> {
    let Proposition::RelationHolds { subject, relation, object } = proposition else {
        return ReasoningOutcome::InternalError("relation verdict requires a relation proposition".to_string());
    };
    let negative = format!(
        "NegativeObjectPropertyAssertion(<{}> <{}> <{}>)",
        relation_iri(relation),
        entity_iri(subject),
        entity_iri(object)
    );
    match reasoner.consistent_with(negative) {
        ReasoningOutcome::Complete(false) => ReasoningOutcome::Complete(Verdict::Entailed(
            RustdlReasoner::explanation(proposition, "rustdl completed the negative property consistency check"),
        )),
        ReasoningOutcome::Complete(true) => {
            let positive = format!(
                "ObjectPropertyAssertion(<{}> <{}> <{}>)",
                relation_iri(relation),
                entity_iri(subject),
                entity_iri(object)
            );
            match reasoner.consistent_with(positive) {
                ReasoningOutcome::Complete(false) => {
                    ReasoningOutcome::Complete(Verdict::Contradicted(RustdlReasoner::explanation(
                        proposition,
                        "rustdl completed the positive property consistency check",
                    )))
                }
                ReasoningOutcome::Complete(true) => ReasoningOutcome::Complete(RustdlReasoner::unknown(
                    proposition,
                    "neither property assertion nor its negation is entailed",
                )),
                outcome => outcome::propagate(outcome),
            }
        }
        outcome => outcome::propagate(outcome),
    }
}
