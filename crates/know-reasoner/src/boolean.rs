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

use std::collections::HashMap;

use know_core::{AxiomId, ConceptId, Diagnostic, EntityId};
use know_ontology::{Axiom, ConceptExpr, ConceptStatus, KnowledgeModule};

use crate::sat::{CnfBuilder, Formula, Var};
use crate::{InconsistencyReport, Proposition, UnsupportedFeature};

// ---------------------------------------------------------------------------
// Translated axioms
// ---------------------------------------------------------------------------

mod query;
mod reasoner_impl;

#[derive(Debug, Clone)]
pub(super) struct TranslatedAxiom {
    pub(super) id: AxiomId,
    pub(super) formula: Formula,
    /// Proposition form for explanation premises, where one exists.
    pub(super) as_proposition: Option<Proposition>,
}

// ---------------------------------------------------------------------------
// Reasoner
// ---------------------------------------------------------------------------

pub struct BooleanReasoner {
    /// Internalized TBox: subclass/equivalence/disjointness axioms plus
    /// definitional equivalences.
    pub(super) tbox: Vec<TranslatedAxiom>,
    /// Per-entity class assertions (positive and negative).
    pub(super) abox: HashMap<EntityId, Vec<TranslatedAxiom>>,
    pub(super) entities: Vec<EntityId>,
    pub(super) vars: HashMap<ConceptId, Var>,
    pub(super) var_names: Vec<ConceptId>,
    /// Set when the module itself uses constructs outside the Boolean
    /// fragment; every query then reports Unsupported.
    pub(super) unsupported: Option<UnsupportedFeature>,
    /// Computed once at construction.
    pub(super) inconsistency: Option<InconsistencyReport>,
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
            Axiom::SubclassOf { child, parent } => match (self.try_formula(child), self.try_formula(parent)) {
                (Some(c), Some(p)) => self.tbox.push(TranslatedAxiom {
                    id: id.clone(),
                    formula: Formula::implies(c, p),
                    as_proposition: Some(Proposition::SubclassOf { child: child.clone(), parent: parent.clone() }),
                }),
                _ => unsupported(self, "relational subclass axiom"),
            },
            Axiom::EquivalentClasses { classes } => {
                let formulas: Option<Vec<Formula>> = classes.iter().map(|c| self.try_formula(c)).collect();
                match formulas {
                    Some(fs) if fs.len() >= 2 => {
                        // Chain of iffs is logically equivalent to full pairwise.
                        let parts: Vec<Formula> =
                            fs.windows(2).map(|w| Formula::iff(w[0].clone(), w[1].clone())).collect();
                        self.tbox.push(TranslatedAxiom {
                            id: id.clone(),
                            formula: Formula::And(parts),
                            as_proposition: (classes.len() == 2).then(|| Proposition::Equivalent {
                                left: classes[0].clone(),
                                right: classes[1].clone(),
                            }),
                        });
                    }
                    Some(_) => {} // fewer than 2 classes constrains nothing
                    None => unsupported(self, "relational equivalence axiom"),
                }
            }
            Axiom::DisjointClasses { classes } => {
                let formulas: Option<Vec<Formula>> = classes.iter().map(|c| self.try_formula(c)).collect();
                match formulas {
                    Some(fs) => {
                        let mut parts = vec![];
                        for i in 0..fs.len() {
                            for j in (i + 1)..fs.len() {
                                parts.push(Formula::not(Formula::And(vec![fs[i].clone(), fs[j].clone()])));
                            }
                        }
                        if !parts.is_empty() {
                            self.tbox.push(TranslatedAxiom {
                                id: id.clone(),
                                formula: Formula::And(parts),
                                as_proposition: (classes.len() == 2).then(|| Proposition::Disjoint {
                                    left: classes[0].clone(),
                                    right: classes[1].clone(),
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

    fn push_abox(&mut self, entity: &EntityId, id: &AxiomId, formula: Formula, class: ConceptExpr, negative: bool) {
        let class = if negative { ConceptExpr::not(class) } else { class };
        self.abox.entry(entity.clone()).or_default().push(TranslatedAxiom {
            id: id.clone(),
            formula,
            as_proposition: Some(Proposition::ClassMembership { entity: entity.clone(), class }),
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
            ConceptExpr::And(parts) => {
                Some(Formula::And(parts.iter().map(|p| self.try_formula(p)).collect::<Option<_>>()?))
            }
            ConceptExpr::Or(parts) => {
                Some(Formula::Or(parts.iter().map(|p| self.try_formula(p)).collect::<Option<_>>()?))
            }
            ConceptExpr::Not(inner) => Some(Formula::not(self.try_formula(inner)?)),
            ConceptExpr::Exists { .. } | ConceptExpr::ForAll { .. } => None,
        }
    }

    // -- SAT plumbing --------------------------------------------------------

    pub(super) fn solve(&self, axioms: &[&TranslatedAxiom], extra: &[&Formula]) -> Option<Vec<bool>> {
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
    pub(super) fn refute(&self, axioms: &[&TranslatedAxiom], extra: &[&Formula]) -> Result<Vec<AxiomId>, Vec<bool>> {
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

    pub(super) fn tbox_refs(&self) -> Vec<&TranslatedAxiom> {
        self.tbox.iter().collect()
    }

    pub(super) fn tbox_and_entity_refs(&self, entity: &EntityId) -> Vec<&TranslatedAxiom> {
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
                explanation: Some("the TBox axioms admit no possible individual at all".to_string()),
            });
        }
        for entity in &self.entities {
            if let Ok(core) = self.refute(&self.tbox_and_entity_refs(entity), &[]) {
                return Some(InconsistencyReport {
                    conflicting_axioms: core,
                    explanation: Some(format!("the assertions about entity {} cannot all hold together", entity.0)),
                });
            }
        }
        None
    }

}

pub(super) enum QueryFault {
    Relational,
    UnknownConcepts(Vec<Diagnostic>),
}

