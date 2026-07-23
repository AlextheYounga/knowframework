//! Stage 2: complete reasoning for the Boolean (role-free) concept fragment.
//!
//! ## Why SAT is sound and complete here
//!
//! Without relational restrictions, ALC constraints on distinct individuals
//! are independent: a TBox axiom `C SUBCLASS_OF D` constrains each domain
//! element separately, so a concept expression `C` is satisfiable w.r.t. the
//! TBox iff there is a single truth assignment to named concepts satisfying
//! (internalized TBox) AND C. Each propositional model corresponds to one
//! domain element's concept membership pattern, and any set of such patterns
//! assembles into a DL model. Hence:
//!
//!   satisfiable(C)        ⟺  SAT( T ∧ C )
//!   entailed(C ⊑ D)       ⟺  UNSAT( T ∧ C ∧ ¬D )
//!   member(e, C) entailed ⟺  UNSAT( T ∧ ABox(e) ∧ ¬C )
//!   KB consistent         ⟺  SAT(T) and, for each entity e, SAT( T ∧ ABox(e) )
//!
//! (SAT(T) is required because OWL interpretation domains are non-empty.)
//!
//! ## Verdict semantics
//!
//! - `SubclassOf`/`Equivalent`/`Disjoint`: `Entailed` by refutation;
//!   `Contradicted` when an ABox entity is a provable counterexample;
//!   otherwise `Unknown` with a countermodel in the explanation.
//! - `Satisfiable`: `Entailed` = provably satisfiable (a witness model
//!   exists), `Contradicted` = provably unsatisfiable. Never `Unknown`:
//!   satisfiability is decided outright in this fragment.
//! - Any query while the KB is inconsistent returns `Inconsistent` rather
//!   than the classical "everything is entailed", which is never useful.
//!
//! Definitions with status `Defined` are necessary-and-sufficient, so each
//! contributes an equivalence `id ↔ definition` (synthetic axiom ID
//! `definition:<concept-id>`). `Deprecated` definitions are retained as data
//! but contribute no axioms.

use std::collections::{HashMap, HashSet};

use know_core::{AxiomId, ConceptId, Diagnostic, EntityId, codes};
use know_ontology::{Axiom, ConceptExpr, ConceptStatus, KnowledgeModule};

use crate::sat::{CnfBuilder, Formula, Var};
use crate::{
    ClassificationResult, Explanation, InconsistencyReport, InferenceRule, InferenceStep,
    Proposition, Reasoner, ReasoningOutcome, UnknownExplanation, UnsupportedFeature, Verdict,
};

// ---------------------------------------------------------------------------
// Translated axioms
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct TranslatedAxiom {
    id: AxiomId,
    formula: Formula,
    /// Proposition form for explanation premises, where one exists.
    as_proposition: Option<Proposition>,
}

// ---------------------------------------------------------------------------
// Reasoner
// ---------------------------------------------------------------------------

pub struct BooleanReasoner {
    /// Internalized TBox: subclass/equivalence/disjointness axioms plus
    /// definitional equivalences.
    tbox: Vec<TranslatedAxiom>,
    /// Per-entity class assertions (positive and negative).
    abox: HashMap<EntityId, Vec<TranslatedAxiom>>,
    entities: Vec<EntityId>,
    vars: HashMap<ConceptId, Var>,
    var_names: Vec<ConceptId>,
    /// Set when the module itself uses constructs outside the Boolean
    /// fragment; every query then reports Unsupported.
    unsupported: Option<UnsupportedFeature>,
    /// Computed once at construction.
    inconsistency: Option<InconsistencyReport>,
}

impl BooleanReasoner {
    pub fn new(module: &KnowledgeModule) -> Self {
        let mut vars = HashMap::new();
        let mut var_names = vec![];
        for c in &module.concepts {
            let v = var_names.len() as Var;
            vars.insert(c.id.clone(), v);
            var_names.push(c.id.clone());
        }

        let mut reasoner = Self {
            tbox: vec![],
            abox: HashMap::new(),
            entities: module.entities.iter().map(|e| e.id.clone()).collect(),
            vars,
            var_names,
            unsupported: None,
            inconsistency: None,
        };

        reasoner.translate_module(module);
        if reasoner.unsupported.is_none() {
            reasoner.inconsistency = reasoner.find_inconsistency();
        }
        reasoner
    }

    // -- Translation --------------------------------------------------------

    fn translate_module(&mut self, module: &KnowledgeModule) {
        for c in &module.concepts {
            if c.status != ConceptStatus::Defined {
                continue;
            }
            let Some(def) = &c.definition else { continue };
            let Some(def_f) = self.try_formula(def) else {
                self.mark_unsupported(format!("definition of {}", c.id.0));
                continue;
            };
            let named = Formula::Var(self.vars[&c.id]);
            self.tbox.push(TranslatedAxiom {
                id: AxiomId(format!("definition:{}", c.id.0)),
                formula: Formula::iff(named, def_f),
                as_proposition: Some(Proposition::Equivalent {
                    left: ConceptExpr::Named(c.id.clone()),
                    right: def.clone(),
                }),
            });
        }

        for annotated in &module.axioms {
            self.translate_axiom(&annotated.id, &annotated.axiom);
        }
    }

    fn translate_axiom(&mut self, id: &AxiomId, axiom: &Axiom) {
        let unsupported = |this: &mut Self, what: &str| {
            this.mark_unsupported(format!("{what} (axiom {})", id.0));
        };

        match axiom {
            Axiom::SubclassOf { child, parent } => {
                match (self.try_formula(child), self.try_formula(parent)) {
                    (Some(c), Some(p)) => self.tbox.push(TranslatedAxiom {
                        id: id.clone(),
                        formula: Formula::implies(c, p),
                        as_proposition: Some(Proposition::SubclassOf {
                            child: child.clone(),
                            parent: parent.clone(),
                        }),
                    }),
                    _ => unsupported(self, "relational subclass axiom"),
                }
            }
            Axiom::EquivalentClasses { classes } => {
                let formulas: Option<Vec<Formula>> =
                    classes.iter().map(|c| self.try_formula(c)).collect();
                match formulas {
                    Some(fs) if fs.len() >= 2 => {
                        // Chain of iffs is logically equivalent to full pairwise.
                        let parts: Vec<Formula> = fs
                            .windows(2)
                            .map(|w| Formula::iff(w[0].clone(), w[1].clone()))
                            .collect();
                        self.tbox.push(TranslatedAxiom {
                            id: id.clone(),
                            formula: Formula::And(parts),
                            as_proposition: (classes.len() == 2).then(|| {
                                Proposition::Equivalent {
                                    left: classes[0].clone(),
                                    right: classes[1].clone(),
                                }
                            }),
                        });
                    }
                    Some(_) => {} // fewer than 2 classes constrains nothing
                    None => unsupported(self, "relational equivalence axiom"),
                }
            }
            Axiom::DisjointClasses { classes } => {
                let formulas: Option<Vec<Formula>> =
                    classes.iter().map(|c| self.try_formula(c)).collect();
                match formulas {
                    Some(fs) => {
                        let mut parts = vec![];
                        for i in 0..fs.len() {
                            for j in (i + 1)..fs.len() {
                                parts.push(Formula::not(Formula::And(vec![
                                    fs[i].clone(),
                                    fs[j].clone(),
                                ])));
                            }
                        }
                        if !parts.is_empty() {
                            self.tbox.push(TranslatedAxiom {
                                id: id.clone(),
                                formula: Formula::And(parts),
                                as_proposition: (classes.len() == 2).then(|| {
                                    Proposition::Disjoint {
                                        left: classes[0].clone(),
                                        right: classes[1].clone(),
                                    }
                                }),
                            });
                        }
                    }
                    None => unsupported(self, "relational disjointness axiom"),
                }
            }
            Axiom::ClassAssertion { entity, class } => match self.try_formula(class) {
                Some(f) => self.push_abox(entity, id, f, class.clone(), false),
                None => unsupported(self, "relational class assertion"),
            },
            Axiom::NegativeClassAssertion { entity, class } => match self.try_formula(class) {
                Some(f) => self.push_abox(entity, id, Formula::not(f), class.clone(), true),
                None => unsupported(self, "relational negative class assertion"),
            },
            Axiom::RelationAssertion { .. } | Axiom::NegativeRelationAssertion { .. } => {
                unsupported(self, "relation assertion");
            }
        }
    }

    fn push_abox(
        &mut self,
        entity: &EntityId,
        id: &AxiomId,
        formula: Formula,
        class: ConceptExpr,
        negative: bool,
    ) {
        let class = if negative { ConceptExpr::not(class) } else { class };
        self.abox.entry(entity.clone()).or_default().push(TranslatedAxiom {
            id: id.clone(),
            formula,
            as_proposition: Some(Proposition::ClassMembership {
                entity: entity.clone(),
                class,
            }),
        });
    }

    fn mark_unsupported(&mut self, feature: String) {
        if self.unsupported.is_none() {
            self.unsupported = Some(UnsupportedFeature {
                feature,
                advice: Some(
                    "relational constructs require the Stage 3 ALC reasoner, \
                     which is not yet implemented"
                        .to_string(),
                ),
            });
        }
    }

    /// None if the expression uses relational restrictions.
    fn try_formula(&self, expr: &ConceptExpr) -> Option<Formula> {
        match expr {
            ConceptExpr::Named(id) => Some(Formula::Var(*self.vars.get(id)?)),
            ConceptExpr::And(parts) => Some(Formula::And(
                parts.iter().map(|p| self.try_formula(p)).collect::<Option<_>>()?,
            )),
            ConceptExpr::Or(parts) => Some(Formula::Or(
                parts.iter().map(|p| self.try_formula(p)).collect::<Option<_>>()?,
            )),
            ConceptExpr::Not(inner) => Some(Formula::not(self.try_formula(inner)?)),
            ConceptExpr::Exists { .. } | ConceptExpr::ForAll { .. } => None,
        }
    }

    // -- SAT plumbing --------------------------------------------------------

    fn solve(&self, axioms: &[&TranslatedAxiom], extra: &[&Formula]) -> Option<Vec<bool>> {
        let mut builder = CnfBuilder::new(self.var_names.len() as Var);
        for a in axioms {
            builder.assert(&a.formula);
        }
        for f in extra {
            builder.assert(f);
        }
        builder.solve()
    }

    /// Try to refute `extra` under the given axioms.
    ///
    /// Returns `Ok(minimal core of axiom IDs)` if axioms ∧ extra is
    /// unsatisfiable, or `Err(model)` with a satisfying assignment otherwise.
    fn refute(
        &self,
        axioms: &[&TranslatedAxiom],
        extra: &[&Formula],
    ) -> Result<Vec<AxiomId>, Vec<bool>> {
        if let Some(model) = self.solve(axioms, extra) {
            return Err(model);
        }
        // Deletion-based minimization: drop axioms one at a time, keeping the
        // removal whenever the remainder still refutes. Quadratic in axiom
        // count, which is fine at V1 scale (Stage 5 owns performance).
        let mut kept: Vec<&TranslatedAxiom> = axioms.to_vec();
        let mut i = 0;
        while i < kept.len() {
            let mut candidate = kept.clone();
            candidate.remove(i);
            if self.solve(&candidate, extra).is_none() {
                kept = candidate;
            } else {
                i += 1;
            }
        }
        Ok(kept.into_iter().map(|a| a.id.clone()).collect())
    }

    fn tbox_refs(&self) -> Vec<&TranslatedAxiom> {
        self.tbox.iter().collect()
    }

    fn tbox_and_entity_refs(&self, entity: &EntityId) -> Vec<&TranslatedAxiom> {
        let mut refs = self.tbox_refs();
        if let Some(assertions) = self.abox.get(entity) {
            refs.extend(assertions.iter());
        }
        refs
    }

    fn find_inconsistency(&self) -> Option<InconsistencyReport> {
        // Non-empty domain: some membership pattern must be possible at all.
        if let Ok(core) = self.refute(&self.tbox_refs(), &[]) {
            return Some(InconsistencyReport {
                conflicting_axioms: core,
                explanation: Some(
                    "the TBox axioms admit no possible individual at all".to_string(),
                ),
            });
        }
        for entity in &self.entities {
            if let Ok(core) = self.refute(&self.tbox_and_entity_refs(entity), &[]) {
                return Some(InconsistencyReport {
                    conflicting_axioms: core,
                    explanation: Some(format!(
                        "the assertions about entity {} cannot all hold together",
                        entity.0
                    )),
                });
            }
        }
        None
    }

    // -- Query helpers -------------------------------------------------------

    /// True when `entity ∈ class` holds in every model of the KB.
    fn membership_entailed(&self, entity: &EntityId, class: &Formula) -> Option<Vec<AxiomId>> {
        let negated = Formula::not(class.clone());
        self.refute(&self.tbox_and_entity_refs(entity), &[&negated]).ok()
    }

    fn explanation(
        &self,
        conclusion: Proposition,
        core: Vec<AxiomId>,
        notes: Option<String>,
    ) -> Explanation {
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
        let steps = vec![InferenceStep {
            rule: InferenceRule::BooleanRefutation,
            premises,
            conclusion: conclusion.clone(),
        }];
        Explanation { conclusion, supporting_axioms: core, steps, notes }
    }

    /// Render a countermodel as the membership pattern of one individual,
    /// restricted to the concepts relevant to the query.
    fn describe_model(&self, model: &[bool], relevant: &[ConceptId]) -> String {
        let mut seen = HashSet::new();
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

    /// Validate that a query expression stays inside the module's vocabulary
    /// and the Boolean fragment. Returns the formula, or the verdict/outcome
    /// to report instead.
    fn query_formula(&self, expr: &ConceptExpr) -> Result<Formula, QueryFault> {
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

    fn known_entity(&self, entity: &EntityId) -> Result<(), QueryFault> {
        if self.entities.contains(entity) {
            Ok(())
        } else {
            Err(QueryFault::UnknownConcepts(vec![Diagnostic::error(
                codes::UNRESOLVED_ENTITY,
                format!("query references entity {} not declared in the module", entity.0),
            )]))
        }
    }

    // -- Verdicts ------------------------------------------------------------

    fn subclass_verdict(
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
                // A provable ABox counterexample makes the subclass claim false.
                for entity in &self.entities {
                    let in_child = self.membership_entailed(entity, &child_f);
                    let out_of_parent = self.membership_entailed(entity, &not_parent);
                    if let (Some(mut core_a), Some(core_b)) = (in_child, out_of_parent) {
                        core_a.extend(core_b);
                        core_a.dedup();
                        return Ok(Verdict::Contradicted(self.explanation(
                            proposition.clone(),
                            core_a,
                            Some(format!(
                                "entity {} is provably {child} but provably NOT {parent}",
                                entity.0
                            )),
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

    fn verdict(&self, proposition: &Proposition) -> Result<Verdict, QueryFault> {
        match proposition {
            Proposition::SubclassOf { child, parent } => {
                self.subclass_verdict(proposition, child, parent)
            }

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
                        Verdict::Contradicted(self.explanation(
                            proposition.clone(),
                            e.supporting_axioms,
                            e.notes,
                        ))
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
                    Ok(core) => {
                        Ok(Verdict::Entailed(self.explanation(proposition.clone(), core, None)))
                    }
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
                                    Some(format!(
                                        "entity {} is provably both {left} and {right}",
                                        entity.0
                                    )),
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
                    return Ok(Verdict::Entailed(self.explanation(
                        proposition.clone(),
                        core,
                        None,
                    )));
                }
                match self.refute(&axioms, &[&class_f]) {
                    Ok(core) => Ok(Verdict::Contradicted(self.explanation(
                        proposition.clone(),
                        core,
                        None,
                    ))),
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

enum QueryFault {
    Relational,
    UnknownConcepts(Vec<Diagnostic>),
}

impl Reasoner for BooleanReasoner {
    fn query(&self, proposition: &Proposition) -> ReasoningOutcome<Verdict> {
        if let Some(u) = &self.unsupported {
            return ReasoningOutcome::Unsupported(u.clone());
        }
        if let Some(report) = &self.inconsistency {
            return ReasoningOutcome::Complete(Verdict::Inconsistent(report.clone()));
        }
        match self.verdict(proposition) {
            Ok(v) => ReasoningOutcome::Complete(v),
            Err(QueryFault::Relational) => ReasoningOutcome::Unsupported(UnsupportedFeature {
                feature: format!("relational proposition: {proposition}"),
                advice: Some("requires the Stage 3 ALC reasoner".to_string()),
            }),
            Err(QueryFault::UnknownConcepts(diags)) => {
                ReasoningOutcome::Complete(Verdict::IllTyped(diags))
            }
        }
    }

    fn classify(&self, concept: &ConceptExpr) -> ReasoningOutcome<ClassificationResult> {
        if let Some(u) = &self.unsupported {
            return ReasoningOutcome::Unsupported(u.clone());
        }
        let expr_f = match self.query_formula(concept) {
            Ok(f) => f,
            Err(QueryFault::Relational) => {
                return ReasoningOutcome::Unsupported(UnsupportedFeature {
                    feature: "classification of relational expression".to_string(),
                    advice: Some("requires the Stage 3 ALC reasoner".to_string()),
                });
            }
            Err(QueryFault::UnknownConcepts(diags)) => {
                return ReasoningOutcome::InternalError(format!(
                    "cannot classify: {}",
                    diags.iter().map(|d| d.message.as_str()).collect::<Vec<_>>().join("; ")
                ));
            }
        };

        let tbox = self.tbox_refs();
        let entails = |sub: &Formula, sup: &Formula| -> bool {
            let negated = Formula::not(sup.clone());
            self.solve(&tbox, &[sub, &negated]).is_none()
        };

        let self_id = match concept {
            ConceptExpr::Named(id) => Some(id.clone()),
            _ => None,
        };

        let mut supers = vec![];
        let mut subs = vec![];
        let mut equivalents = vec![];
        for id in &self.var_names {
            if Some(id) == self_id.as_ref() {
                continue;
            }
            let named_f = Formula::Var(self.vars[id]);
            let up = entails(&expr_f, &named_f);
            let down = entails(&named_f, &expr_f);
            match (up, down) {
                (true, true) => equivalents.push(id.clone()),
                (true, false) => supers.push(id.clone()),
                (false, true) => subs.push(id.clone()),
                (false, false) => {}
            }
        }

        // Reduce to direct neighbours: a superclass is direct when no other
        // strict superclass sits between it and the queried concept.
        let strictly_between = |a: &ConceptId, b: &ConceptId| -> bool {
            let a_f = Formula::Var(self.vars[a]);
            let b_f = Formula::Var(self.vars[b]);
            entails(&a_f, &b_f) && !entails(&b_f, &a_f)
        };
        let direct_superclasses = supers
            .iter()
            .filter(|n| !supers.iter().any(|m| m != *n && strictly_between(m, n)))
            .cloned()
            .collect();
        let direct_subclasses = subs
            .iter()
            .filter(|n| !subs.iter().any(|m| m != *n && strictly_between(n, m)))
            .cloned()
            .collect();

        ReasoningOutcome::Complete(ClassificationResult {
            direct_superclasses,
            direct_subclasses,
            equivalent_classes: equivalents,
        })
    }

    fn is_consistent(&self) -> ReasoningOutcome<bool> {
        if let Some(u) = &self.unsupported {
            return ReasoningOutcome::Unsupported(u.clone());
        }
        ReasoningOutcome::Complete(self.inconsistency.is_none())
    }

    fn is_satisfiable(&self, concept: &ConceptExpr) -> ReasoningOutcome<bool> {
        if let Some(u) = &self.unsupported {
            return ReasoningOutcome::Unsupported(u.clone());
        }
        match self.query_formula(concept) {
            Ok(f) => ReasoningOutcome::Complete(self.solve(&self.tbox_refs(), &[&f]).is_some()),
            Err(QueryFault::Relational) => ReasoningOutcome::Unsupported(UnsupportedFeature {
                feature: "satisfiability of relational expression".to_string(),
                advice: Some("requires the Stage 3 ALC reasoner".to_string()),
            }),
            Err(QueryFault::UnknownConcepts(_)) => {
                // An undeclared concept is unconstrained, hence trivially
                // satisfiable, but answering would hide the typo. Report it.
                ReasoningOutcome::InternalError(
                    "expression references concepts not declared in the module".to_string(),
                )
            }
        }
    }
}
