//! A small, complete Boolean decision procedure: propositional formulas,
//! Tseitin CNF conversion, and a DPLL solver with unit propagation.
//!
//! This is the Stage 2 backend. It is deliberately simple: correctness over
//! performance (architecture plan, Stage 5 handles optimization). Ontologies
//! at this stage are tiny; DPLL with unit propagation is more than enough.

// ---------------------------------------------------------------------------
// Formulas
// ---------------------------------------------------------------------------

pub type Var = u32;

#[derive(Debug, Clone, PartialEq)]
pub enum Formula {
    True,
    False,
    Var(Var),
    Not(Box<Formula>),
    And(Vec<Formula>),
    Or(Vec<Formula>),
}

impl Formula {
    // Logical negation constructor; intentionally shadows the name `not`.
    #[allow(clippy::should_implement_trait)]
    pub fn not(f: Formula) -> Formula {
        Formula::Not(Box::new(f))
    }

    /// child → parent, i.e. ¬child ∨ parent.
    pub fn implies(child: Formula, parent: Formula) -> Formula {
        Formula::Or(vec![Formula::not(child), parent])
    }

    /// left ↔ right.
    pub fn iff(left: Formula, right: Formula) -> Formula {
        Formula::And(vec![Formula::implies(left.clone(), right.clone()), Formula::implies(right, left)])
    }
}

// ---------------------------------------------------------------------------
// Literals and CNF
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lit {
    pub var: Var,
    pub positive: bool,
}

impl Lit {
    pub fn pos(var: Var) -> Self {
        Self { var, positive: true }
    }

    pub fn neg(var: Var) -> Self {
        Self { var, positive: false }
    }

    fn negated(self) -> Self {
        Self { var: self.var, positive: !self.positive }
    }
}

/// Accumulates clauses over a shared variable space. Tseitin conversion
/// allocates fresh variables beyond the caller's, so the same builder must be
/// used for every formula participating in one satisfiability question.
pub struct CnfBuilder {
    next_var: Var,
    clauses: Vec<Vec<Lit>>,
}

impl CnfBuilder {
    /// `first_free_var` must be greater than every variable used in the
    /// formulas that will be added.
    pub fn new(first_free_var: Var) -> Self {
        Self { next_var: first_free_var, clauses: vec![] }
    }

    /// Assert that `formula` is true.
    pub fn assert(&mut self, formula: &Formula) {
        match simplify(formula) {
            Formula::True => {}
            Formula::False => self.clauses.push(vec![]),
            other => {
                let lit = self.tseitin(&other);
                self.clauses.push(vec![lit]);
            }
        }
    }

    /// Tseitin-encode a constant-free subformula, returning the literal that
    /// represents it. `simplify` guarantees no True/False remains below here.
    fn tseitin(&mut self, formula: &Formula) -> Lit {
        match formula {
            Formula::True | Formula::False => unreachable!("constants removed by simplify"),
            Formula::Var(v) => Lit::pos(*v),
            Formula::Not(inner) => self.tseitin(inner).negated(),
            Formula::And(parts) => {
                let lits: Vec<Lit> = parts.iter().map(|p| self.tseitin(p)).collect();
                let out = self.fresh();
                // out → each lit
                for l in &lits {
                    self.clauses.push(vec![out.negated(), *l]);
                }
                // all lits → out
                let mut clause: Vec<Lit> = lits.iter().map(|l| l.negated()).collect();
                clause.push(out);
                self.clauses.push(clause);
                out
            }
            Formula::Or(parts) => {
                let lits: Vec<Lit> = parts.iter().map(|p| self.tseitin(p)).collect();
                let out = self.fresh();
                // out → some lit
                let mut clause = vec![out.negated()];
                clause.extend(lits.iter().copied());
                self.clauses.push(clause);
                // each lit → out
                for l in &lits {
                    self.clauses.push(vec![l.negated(), out]);
                }
                out
            }
        }
    }

    fn fresh(&mut self) -> Lit {
        let v = self.next_var;
        self.next_var += 1;
        Lit::pos(v)
    }

    /// Solve the accumulated clauses. Returns a model (indexed by variable)
    /// if satisfiable, or None if unsatisfiable.
    pub fn solve(&self) -> Option<Vec<bool>> {
        let num_vars = self.next_var as usize;
        let mut assignment: Vec<Option<bool>> = vec![None; num_vars];
        if dpll(&self.clauses, &mut assignment) {
            Some(assignment.into_iter().map(|a| a.unwrap_or(false)).collect())
        } else {
            None
        }
    }
}

/// Fold True/False constants out of a formula. The result either is a bare
/// constant or contains no constants at all — which the Tseitin encoder
/// relies on.
fn simplify(formula: &Formula) -> Formula {
    match formula {
        Formula::True | Formula::False | Formula::Var(_) => formula.clone(),
        Formula::Not(inner) => match simplify(inner) {
            Formula::True => Formula::False,
            Formula::False => Formula::True,
            other => Formula::not(other),
        },
        Formula::And(parts) => {
            let mut kept = vec![];
            for p in parts {
                match simplify(p) {
                    Formula::True => {}
                    Formula::False => return Formula::False,
                    other => kept.push(other),
                }
            }
            match kept.len() {
                0 => Formula::True,
                1 => kept.pop().unwrap(),
                _ => Formula::And(kept),
            }
        }
        Formula::Or(parts) => {
            let mut kept = vec![];
            for p in parts {
                match simplify(p) {
                    Formula::False => {}
                    Formula::True => return Formula::True,
                    other => kept.push(other),
                }
            }
            match kept.len() {
                0 => Formula::False,
                1 => kept.pop().unwrap(),
                _ => Formula::Or(kept),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DPLL
// ---------------------------------------------------------------------------

fn dpll(clauses: &[Vec<Lit>], assignment: &mut Vec<Option<bool>>) -> bool {
    // Unit propagation to fixpoint. Track which vars we set at this level so
    // backtracking can undo them.
    let mut trail: Vec<Var> = vec![];

    loop {
        let mut propagated = false;
        for clause in clauses {
            match clause_status(clause, assignment) {
                ClauseStatus::Satisfied => continue,
                ClauseStatus::Conflict => {
                    undo(assignment, &trail);
                    return false;
                }
                ClauseStatus::Unit(lit) => {
                    assignment[lit.var as usize] = Some(lit.positive);
                    trail.push(lit.var);
                    propagated = true;
                }
                ClauseStatus::Unresolved => continue,
            }
        }
        if !propagated {
            break;
        }
    }

    // Find an unassigned variable to branch on.
    let branch_var = assignment.iter().position(|a| a.is_none());
    let Some(v) = branch_var else {
        return true; // fully assigned with no conflict
    };

    for value in [true, false] {
        assignment[v] = Some(value);
        if dpll(clauses, assignment) {
            return true;
        }
        assignment[v] = None;
    }

    undo(assignment, &trail);
    false
}

fn undo(assignment: &mut [Option<bool>], trail: &[Var]) {
    for v in trail {
        assignment[*v as usize] = None;
    }
}

enum ClauseStatus {
    Satisfied,
    Conflict,
    Unit(Lit),
    Unresolved,
}

fn clause_status(clause: &[Lit], assignment: &[Option<bool>]) -> ClauseStatus {
    let mut unassigned: Option<Lit> = None;
    let mut unassigned_count = 0;

    for lit in clause {
        match assignment[lit.var as usize] {
            Some(v) if v == lit.positive => return ClauseStatus::Satisfied,
            Some(_) => continue,
            None => {
                unassigned = Some(*lit);
                unassigned_count += 1;
            }
        }
    }

    match unassigned_count {
        0 => ClauseStatus::Conflict,
        1 => ClauseStatus::Unit(unassigned.unwrap()),
        _ => ClauseStatus::Unresolved,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sat(formulas: &[Formula], num_vars: Var) -> bool {
        let mut b = CnfBuilder::new(num_vars);
        for f in formulas {
            b.assert(f);
        }
        b.solve().is_some()
    }

    #[test]
    fn single_variable_is_satisfiable() {
        assert!(sat(&[Formula::Var(0)], 1));
    }

    #[test]
    fn contradiction_is_unsatisfiable() {
        assert!(!sat(&[Formula::Var(0), Formula::not(Formula::Var(0))], 1));
    }

    #[test]
    fn and_of_contradictory_parts_is_unsatisfiable() {
        let f = Formula::And(vec![Formula::Var(0), Formula::not(Formula::Var(0))]);
        assert!(!sat(&[f], 1));
    }

    #[test]
    fn or_needs_only_one_branch() {
        let f = Formula::And(vec![Formula::Or(vec![Formula::Var(0), Formula::Var(1)]), Formula::not(Formula::Var(0))]);
        assert!(sat(&[f], 2));
    }

    #[test]
    fn implication_chain_propagates() {
        // a, a→b, b→c, ¬c is unsatisfiable.
        let formulas = [
            Formula::Var(0),
            Formula::implies(Formula::Var(0), Formula::Var(1)),
            Formula::implies(Formula::Var(1), Formula::Var(2)),
            Formula::not(Formula::Var(2)),
        ];
        assert!(!sat(&formulas, 3));
    }

    #[test]
    fn iff_is_symmetric() {
        // a↔b, a, ¬b is unsatisfiable.
        let formulas = [Formula::iff(Formula::Var(0), Formula::Var(1)), Formula::Var(0), Formula::not(Formula::Var(1))];
        assert!(!sat(&formulas, 2));
    }

    #[test]
    fn constants_fold_correctly() {
        assert!(sat(&[Formula::True], 0));
        assert!(!sat(&[Formula::False], 0));
        assert!(sat(&[Formula::Or(vec![Formula::False, Formula::Var(0)])], 1));
        assert!(!sat(&[Formula::And(vec![Formula::True, Formula::False])], 0));
    }

    #[test]
    fn model_respects_assertions() {
        let mut b = CnfBuilder::new(2);
        b.assert(&Formula::Var(0));
        b.assert(&Formula::not(Formula::Var(1)));
        let model = b.solve().expect("satisfiable");
        assert!(model[0]);
        assert!(!model[1]);
    }
}
