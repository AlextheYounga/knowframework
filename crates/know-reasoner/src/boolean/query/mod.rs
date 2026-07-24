use know_core::{AxiomId, ConceptId, Diagnostic, EntityId, codes};
use know_ontology::ConceptExpr;

use crate::{
    Explanation, InferenceRule, InferenceStep, Proposition, UnknownExplanation, Verdict,
    sat::Formula,
};

use super::{BooleanReasoner, QueryFault};

impl BooleanReasoner {
    pub(super) fn membership_entailed(&self, entity: &EntityId, class: &Formula) -> Option<Vec<AxiomId>> {
        let negated = Formula::not(class.clone());
        self.refute(&self.tbox_and_entity_refs(entity), &[&negated]).ok()
    }

    pub(super) fn explanation(&self, conclusion: Proposition, core: Vec<AxiomId>, notes: Option<String>) -> Explanation {
        let premises: Vec<Proposition> = core
            .iter()
            .filter_map(|id| {
                self.tbox
                    .iter()
                    .chain(self.abox.values().flatten())
                    .find(|a| &a.id == id)
                    .and_then(|a| a.as_proposition.clone())
            })
            .collect();
        let steps =
            vec![InferenceStep { rule: InferenceRule::BooleanRefutation, premises, conclusion: conclusion.clone() }];
        Explanation { conclusion, supporting_axioms: core, steps, notes }
    }

    pub(super) fn describe_model(&self, model: &[bool], relevant: &[ConceptId]) -> String {
        let mut seen = std::collections::HashSet::new();
        let parts: Vec<String> = relevant
            .iter()
            .filter(|id| seen.insert((*id).clone()))
            .filter_map(|id| {
                let var = *self.vars.get(id)? as usize;
                Some(if model[var] { id.0.clone() } else { format!("NOT {}", id.0) })
            })
            .collect();
        format!("an individual with {{{}}} is possible", parts.join(", "))
    }

    pub(super) fn query_formula(&self, expr: &ConceptExpr) -> Result<Formula, QueryFault> {
        if expr.uses_relations() {
            return Err(QueryFault::Relational);
        }
        let mut named = vec![];
        expr.named_concepts(&mut named);
        let unknown: Vec<Diagnostic> = named
            .iter()
            .filter(|id| !self.vars.contains_key(id))
            .map(|id| {
                Diagnostic::error(
                    codes::UNRESOLVED_CONCEPT,
                    format!("query references concept {} not declared in the module", id.0),
                )
            })
            .collect();
        if !unknown.is_empty() {
            return Err(QueryFault::UnknownConcepts(unknown));
        }
        Ok(self.try_formula(expr).expect("relational case handled above"))
    }

    pub(super) fn known_entity(&self, entity: &EntityId) -> Result<(), QueryFault> {
        if self.entities.contains(entity) {
            Ok(())
        } else {
            Err(QueryFault::UnknownConcepts(vec![Diagnostic::error(
                codes::UNRESOLVED_ENTITY,
                format!("query references entity {} not declared in the module", entity.0),
            )]))
        }
    }

    pub(super) fn subclass_verdict(
        &self,
        proposition: &Proposition,
        child: &ConceptExpr,
        parent: &ConceptExpr,
    ) -> Result<Verdict, QueryFault> {
        let child_f = self.query_formula(child)?;
        let parent_f = self.query_formula(parent)?;

        let not_parent = Formula::not(parent_f.clone());
        match self.refute(&self.tbox_refs(), &[&child_f, &not_parent]) {
            Ok(core) => Ok(Verdict::Entailed(self.explanation(proposition.clone(), core, None))),
            Err(model) => {
                for entity in &self.entities {
                    let in_child = self.membership_entailed(entity, &child_f);
                    let out_of_parent = self.membership_entailed(entity, &not_parent);
                    if let (Some(mut core_a), Some(core_b)) = (in_child, out_of_parent) {
                        core_a.extend(core_b);
                        core_a.dedup();
                        return Ok(Verdict::Contradicted(self.explanation(
                            proposition.clone(),
                            core_a,
                            Some(format!("entity {} is provably {child} but provably NOT {parent}", entity.0)),
                        )));
                    }
                }

                let mut relevant = vec![];
                child.named_concepts(&mut relevant);
                parent.named_concepts(&mut relevant);
                Ok(Verdict::Unknown(UnknownExplanation {
                    proposition: proposition.clone(),
                    missing: vec![
                        format!(
                            "no axioms force every {child} to be {parent}: {}",
                            self.describe_model(&model, &relevant)
                        ),
                        format!(
                            "no entity is provably {child} yet provably NOT {parent}, \
                             so the negation is not entailed either"
                        ),
                    ],
                }))
            }
        }
    }

    pub(super) fn verdict(&self, proposition: &Proposition) -> Result<Verdict, QueryFault> {
        match proposition {
            Proposition::SubclassOf { child, parent } => self.subclass_verdict(proposition, child, parent),

            Proposition::Equivalent { left, right } => {
                let forward = Proposition::SubclassOf { child: left.clone(), parent: right.clone() };
                let backward = Proposition::SubclassOf { child: right.clone(), parent: left.clone() };
                let fwd = self.subclass_verdict(&forward, left, right)?;
                let bwd = self.subclass_verdict(&backward, right, left)?;
                Ok(match (fwd, bwd) {
                    (Verdict::Entailed(a), Verdict::Entailed(b)) => {
                        let mut core = a.supporting_axioms;
                        core.extend(b.supporting_axioms);
                        core.dedup();
                        Verdict::Entailed(self.explanation(proposition.clone(), core, None))
                    }
                    (Verdict::Contradicted(e), _) | (_, Verdict::Contradicted(e)) => {
                        Verdict::Contradicted(self.explanation(proposition.clone(), e.supporting_axioms, e.notes))
                    }
                    _ => Verdict::Unknown(UnknownExplanation {
                        proposition: proposition.clone(),
                        missing: vec![format!(
                            "equivalence requires both {left} SUBCLASS_OF {right} and \
                             {right} SUBCLASS_OF {left} to be entailed; at least one is open"
                        )],
                    }),
                })
            }

            Proposition::Disjoint { left, right } => {
                let left_f = self.query_formula(left)?;
                let right_f = self.query_formula(right)?;
                match self.refute(&self.tbox_refs(), &[&left_f, &right_f]) {
                    Ok(core) => Ok(Verdict::Entailed(self.explanation(proposition.clone(), core, None))),
                    Err(model) => {
                        for entity in &self.entities {
                            let in_left = self.membership_entailed(entity, &left_f);
                            let in_right = self.membership_entailed(entity, &right_f);
                            if let (Some(mut core_a), Some(core_b)) = (in_left, in_right) {
                                core_a.extend(core_b);
                                core_a.dedup();
                                return Ok(Verdict::Contradicted(self.explanation(
                                    proposition.clone(),
                                    core_a,
                                    Some(format!("entity {} is provably both {left} and {right}", entity.0)),
                                )));
                            }
                        }
                        let mut relevant = vec![];
                        left.named_concepts(&mut relevant);
                        right.named_concepts(&mut relevant);
                        Ok(Verdict::Unknown(UnknownExplanation {
                            proposition: proposition.clone(),
                            missing: vec![
                                format!(
                                    "no axioms make {left} and {right} disjoint: {}",
                                    self.describe_model(&model, &relevant)
                                ),
                                "no entity is provably a member of both".to_string(),
                            ],
                        }))
                    }
                }
            }

            Proposition::Satisfiable { class } => {
                let class_f = self.query_formula(class)?;
                match self.refute(&self.tbox_refs(), &[&class_f]) {
                    Ok(core) => Ok(Verdict::Contradicted(self.explanation(
                        proposition.clone(),
                        core,
                        Some(format!("{class} can have no instances in any model")),
                    ))),
                    Err(model) => {
                        let mut relevant = vec![];
                        class.named_concepts(&mut relevant);
                        Ok(Verdict::Entailed(self.explanation(
                            proposition.clone(),
                            vec![],
                            Some(self.describe_model(&model, &relevant)),
                        )))
                    }
                }
            }

            Proposition::ClassMembership { entity, class } => {
                self.known_entity(entity)?;
                let class_f = self.query_formula(class)?;
                let axioms = self.tbox_and_entity_refs(entity);

                let negated = Formula::not(class_f.clone());
                if let Ok(core) = self.refute(&axioms, &[&negated]) {
                    return Ok(Verdict::Entailed(self.explanation(proposition.clone(), core, None)));
                }
                match self.refute(&axioms, &[&class_f]) {
                    Ok(core) => Ok(Verdict::Contradicted(self.explanation(proposition.clone(), core, None))),
                    Err(_) => Ok(Verdict::Unknown(UnknownExplanation {
                        proposition: proposition.clone(),
                        missing: vec![format!(
                            "the assertions about {} neither force nor exclude {class}; \
                             both remain possible under open-world semantics",
                            entity.0
                        )],
                    })),
                }
            }

            Proposition::Consistent => Ok(Verdict::Entailed(Explanation {
                conclusion: proposition.clone(),
                supporting_axioms: vec![],
                steps: vec![],
                notes: Some(
                    "the TBox admits at least one individual and every entity's \
                     assertions are jointly satisfiable"
                        .to_string(),
                ),
            })),

            Proposition::RelationHolds { .. } => Err(QueryFault::Relational),
        }
    }
}
