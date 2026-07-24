use know_core::ConceptId;
use know_ontology::ConceptExpr;

use crate::{
    ClassificationResult, Proposition, Reasoner, ReasoningOutcome, UnsupportedFeature, Verdict,
};

use super::{BooleanReasoner, QueryFault};
use crate::sat::Formula;

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
            Err(QueryFault::UnknownConcepts(diags)) => ReasoningOutcome::Complete(Verdict::IllTyped(diags)),
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

        let strictly_between = |a: &ConceptId, b: &ConceptId| -> bool {
            let a_f = Formula::Var(self.vars[a]);
            let b_f = Formula::Var(self.vars[b]);
            entails(&a_f, &b_f) && !entails(&b_f, &a_f)
        };
        let direct_superclasses =
            supers.iter().filter(|n| !supers.iter().any(|m| m != *n && strictly_between(m, n))).cloned().collect();
        let direct_subclasses =
            subs.iter().filter(|n| !subs.iter().any(|m| m != *n && strictly_between(n, m))).cloned().collect();

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
                ReasoningOutcome::InternalError("expression references concepts not declared in the module".to_string())
            }
        }
    }
}
