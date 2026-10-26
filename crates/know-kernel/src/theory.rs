use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::{CheckError, Checker, Term};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationKind {
    Symbol,
    Assumption,
    Definition,
    Theorem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub kind: DeclarationKind,
    pub ty: Term,
    pub value: Option<Term>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedTheorem {
    pub name: String,
    pub proposition: Term,
    /// The transitive logical assumptions on which this theorem depends.
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TheoryError {
    #[error("declaration `{0}` already exists")]
    DuplicateDeclaration(String),
    #[error("definition `{0}` is circular")]
    CircularDefinition(String),
    #[error("assumption or theorem type is not a proposition: `{0:?}`")]
    NotAProposition(Term),
    #[error("symbol `{0}` would introduce a proof; use an explicit assumption instead")]
    ProofRequiresAssumption(String),
    #[error(transparent)]
    Check(#[from] CheckError),
}

#[derive(Debug, Clone, Default)]
pub struct Theory {
    declarations: HashMap<String, Declaration>,
}

impl Theory {
    #[must_use]
    pub fn checker(&self) -> Checker<'_> {
        Checker::new(&self.declarations)
    }

    /// Declare a type, entity, predicate, relation, or other non-proof symbol.
    ///
    /// # Errors
    ///
    /// Returns an error when the name already exists or the declared type is
    /// not well-formed in the current theory.
    pub fn declare_symbol(&mut self, name: impl Into<String>, ty: Term) -> Result<(), TheoryError> {
        let name = name.into();
        self.ensure_available(&name)?;
        self.checker().check_type(&ty)?;
        if self.checker().definitionally_equal(&self.checker().infer(&ty)?, &Term::Prop) {
            return Err(TheoryError::ProofRequiresAssumption(name));
        }
        self.declarations.insert(name, Declaration { kind: DeclarationKind::Symbol, ty, value: None });
        Ok(())
    }

    /// Add an explicitly named logical assumption.
    ///
    /// # Errors
    ///
    /// Returns an error when the name exists or the supplied term is not a
    /// well-formed proposition.
    pub fn assume(&mut self, name: impl Into<String>, proposition: Term) -> Result<(), TheoryError> {
        let name = name.into();
        self.ensure_available(&name)?;
        self.ensure_proposition(&proposition)?;
        self.declarations.insert(name, Declaration { kind: DeclarationKind::Assumption, ty: proposition, value: None });
        Ok(())
    }

    /// Add a transparent, non-recursive definition.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicate or circular definitions, or when the
    /// value does not have the declared type.
    pub fn define(&mut self, name: impl Into<String>, ty: Term, value: Term) -> Result<(), TheoryError> {
        let name = name.into();
        self.ensure_available(&name)?;
        let mut constants = HashSet::new();
        value.constants(&mut constants);
        if constants.contains(&name) {
            return Err(TheoryError::CircularDefinition(name));
        }
        self.checker().check_type(&ty)?;
        self.checker().check(&value, &ty)?;
        self.declarations.insert(name, Declaration { kind: DeclarationKind::Definition, ty, value: Some(value) });
        Ok(())
    }

    /// Check and add a theorem, returning its transitive assumption lineage.
    ///
    /// # Errors
    ///
    /// Returns an error when the proposition is malformed, the proof does not
    /// inhabit it, or the theorem name already exists.
    pub fn prove(
        &mut self,
        name: impl Into<String>,
        proposition: Term,
        proof: Term,
    ) -> Result<CheckedTheorem, TheoryError> {
        let name = name.into();
        self.ensure_available(&name)?;
        self.ensure_proposition(&proposition)?;
        self.checker().check(&proof, &proposition)?;

        let mut dependencies = HashSet::new();
        proof.constants(&mut dependencies);
        let mut assumptions = HashSet::new();
        let mut visited = HashSet::new();
        for dependency in dependencies {
            self.collect_assumptions(&dependency, &mut assumptions, &mut visited);
        }
        let mut assumptions: Vec<_> = assumptions.into_iter().collect();
        assumptions.sort();

        self.declarations.insert(
            name.clone(),
            Declaration { kind: DeclarationKind::Theorem, ty: proposition.clone(), value: Some(proof) },
        );
        Ok(CheckedTheorem { name, proposition, assumptions })
    }

    #[must_use]
    pub fn declaration(&self, name: &str) -> Option<&Declaration> {
        self.declarations.get(name)
    }

    fn ensure_available(&self, name: &str) -> Result<(), TheoryError> {
        if self.declarations.contains_key(name) {
            Err(TheoryError::DuplicateDeclaration(name.to_string()))
        } else {
            Ok(())
        }
    }

    fn ensure_proposition(&self, proposition: &Term) -> Result<(), TheoryError> {
        let proposition_type = self.checker().infer(proposition)?;
        if self.checker().definitionally_equal(&proposition_type, &Term::Prop) {
            Ok(())
        } else {
            Err(TheoryError::NotAProposition(proposition.clone()))
        }
    }

    fn collect_assumptions(&self, name: &str, assumptions: &mut HashSet<String>, visited: &mut HashSet<String>) {
        if !visited.insert(name.to_string()) {
            return;
        }
        let Some(declaration) = self.declarations.get(name) else {
            return;
        };
        if declaration.kind == DeclarationKind::Assumption {
            assumptions.insert(name.to_string());
        }
        if let Some(value) = &declaration.value {
            let mut dependencies = HashSet::new();
            value.constants(&mut dependencies);
            for dependency in dependencies {
                self.collect_assumptions(&dependency, assumptions, visited);
            }
        }
    }
}
