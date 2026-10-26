use std::collections::HashMap;

use thiserror::Error;

use crate::{Declaration, Term};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CheckError {
    #[error("unknown constant `{0}`")]
    UnknownConstant(String),
    #[error("bound variable {index} escapes its scope of depth {depth}")]
    EscapedVariable { index: usize, depth: usize },
    #[error("expected a type, but `{term:?}` has type `{actual:?}`")]
    ExpectedType { term: Term, actual: Term },
    #[error("expected a function, but `{term:?}` has type `{actual:?}`")]
    ExpectedFunction { term: Term, actual: Term },
    #[error("type mismatch: expected `{expected:?}`, found `{actual:?}`")]
    TypeMismatch { expected: Term, actual: Term },
}

/// Type checker and normalizer for a fixed declaration environment.
pub struct Checker<'a> {
    declarations: &'a HashMap<String, Declaration>,
}

impl<'a> Checker<'a> {
    #[must_use]
    pub const fn new(declarations: &'a HashMap<String, Declaration>) -> Self {
        Self { declarations }
    }

    /// Infer the type of a closed term.
    ///
    /// # Errors
    ///
    /// Returns an error when the term is unbound, ill-scoped, or ill-typed.
    pub fn infer(&self, term: &Term) -> Result<Term, CheckError> {
        self.infer_in(term, &[])
    }

    /// Check a closed term against an expected type.
    ///
    /// # Errors
    ///
    /// Returns an error when either term is invalid or their types are not
    /// definitionally equal.
    pub fn check(&self, term: &Term, expected: &Term) -> Result<(), CheckError> {
        let actual = self.infer(term)?;
        if self.definitionally_equal(&actual, expected) {
            Ok(())
        } else {
            Err(CheckError::TypeMismatch { expected: expected.clone(), actual })
        }
    }

    /// Check that a closed term is a well-formed type or proposition.
    ///
    /// # Errors
    ///
    /// Returns an error when the term is malformed or does not inhabit a sort.
    pub fn check_type(&self, term: &Term) -> Result<(), CheckError> {
        self.expect_sort(term, &[]).map(|_| ())
    }

    /// Normalize a term by beta reduction and transparent definition unfolding.
    #[must_use]
    pub fn normalize(&self, term: &Term) -> Term {
        match term {
            Term::Constant(name) => self
                .declarations
                .get(name)
                .and_then(|declaration| declaration.value.as_ref())
                .map_or_else(|| term.clone(), |value| self.normalize(value)),
            Term::Apply { function, argument } => {
                let normalized_function = self.normalize(function);
                let normalized_argument = self.normalize(argument);
                if let Term::Lambda { body, .. } = normalized_function {
                    self.normalize(&body.instantiate(&normalized_argument))
                } else {
                    Term::apply(normalized_function, normalized_argument)
                }
            }
            Term::Pi { parameter_type, body } => Term::pi(self.normalize(parameter_type), self.normalize(body)),
            Term::Lambda { parameter_type, body } => Term::lambda(self.normalize(parameter_type), self.normalize(body)),
            Term::Equal { ty, left, right } => {
                Term::equal(self.normalize(ty), self.normalize(left), self.normalize(right))
            }
            Term::Refl(value) => Term::refl(self.normalize(value)),
            Term::Rewrite { equality, motive, proof } => {
                Term::rewrite(self.normalize(equality), self.normalize(motive), self.normalize(proof))
            }
            Term::Prop | Term::Universe(_) | Term::Bound(_) => term.clone(),
        }
    }

    #[must_use]
    pub fn definitionally_equal(&self, left: &Term, right: &Term) -> bool {
        self.normalize(left) == self.normalize(right)
    }

    #[expect(clippy::too_many_lines, reason = "the complete typing relation is clearer as one structural match")]
    fn infer_in(&self, term: &Term, context: &[Term]) -> Result<Term, CheckError> {
        match term {
            Term::Prop => Ok(Term::Universe(0)),
            Term::Universe(level) => Ok(Term::Universe(level + 1)),
            Term::Bound(index) => context
                .get(*index)
                .map(|ty| ty.shift(isize::try_from(index + 1).unwrap_or(isize::MAX), 0))
                .ok_or(CheckError::EscapedVariable { index: *index, depth: context.len() }),
            Term::Constant(name) => self
                .declarations
                .get(name)
                .map(|declaration| declaration.ty.clone())
                .ok_or_else(|| CheckError::UnknownConstant(name.clone())),
            Term::Pi { parameter_type, body } => {
                let parameter_universe = self.expect_sort(parameter_type, context)?;
                let mut extended = Vec::with_capacity(context.len() + 1);
                extended.push((**parameter_type).clone());
                extended.extend_from_slice(context);
                let body_type = self.infer_in(body, &extended)?;
                if self.definitionally_equal(&body_type, &Term::Prop) {
                    Ok(Term::Prop)
                } else if let Term::Universe(body_universe) = self.normalize(&body_type) {
                    Ok(Term::Universe(parameter_universe.max(body_universe)))
                } else {
                    Err(CheckError::ExpectedType { term: (**body).clone(), actual: body_type })
                }
            }
            Term::Lambda { parameter_type, body } => {
                self.expect_sort(parameter_type, context)?;
                let mut extended = Vec::with_capacity(context.len() + 1);
                extended.push((**parameter_type).clone());
                extended.extend_from_slice(context);
                let body_type = self.infer_in(body, &extended)?;
                Ok(Term::pi((**parameter_type).clone(), body_type))
            }
            Term::Apply { function, argument } => {
                let function_type = self.normalize(&self.infer_in(function, context)?);
                let Term::Pi { parameter_type, body } = function_type else {
                    return Err(CheckError::ExpectedFunction { term: (**function).clone(), actual: function_type });
                };
                let argument_type = self.infer_in(argument, context)?;
                if !self.definitionally_equal(&argument_type, &parameter_type) {
                    return Err(CheckError::TypeMismatch { expected: *parameter_type, actual: argument_type });
                }
                Ok(body.instantiate(argument))
            }
            Term::Equal { ty, left, right } => {
                self.expect_sort(ty, context)?;
                let left_type = self.infer_in(left, context)?;
                if !self.definitionally_equal(&left_type, ty) {
                    return Err(CheckError::TypeMismatch { expected: (**ty).clone(), actual: left_type });
                }
                let right_type = self.infer_in(right, context)?;
                if !self.definitionally_equal(&right_type, ty) {
                    return Err(CheckError::TypeMismatch { expected: (**ty).clone(), actual: right_type });
                }
                Ok(Term::Prop)
            }
            Term::Refl(value) => {
                let ty = self.infer_in(value, context)?;
                Ok(Term::equal(ty, (**value).clone(), (**value).clone()))
            }
            Term::Rewrite { equality, motive, proof } => {
                let equality_type = self.normalize(&self.infer_in(equality, context)?);
                let Term::Equal { ty, left, right } = equality_type else {
                    return Err(CheckError::TypeMismatch {
                        expected: Term::constant("equality proof"),
                        actual: equality_type,
                    });
                };
                let expected_motive_type = Term::function((*ty).clone(), Term::Prop);
                let motive_type = self.infer_in(motive, context)?;
                if !self.definitionally_equal(&motive_type, &expected_motive_type) {
                    return Err(CheckError::TypeMismatch { expected: expected_motive_type, actual: motive_type });
                }
                let left_proposition = Term::apply((**motive).clone(), (*left).clone());
                let proof_type = self.infer_in(proof, context)?;
                if !self.definitionally_equal(&proof_type, &left_proposition) {
                    return Err(CheckError::TypeMismatch { expected: left_proposition, actual: proof_type });
                }
                Ok(Term::apply((**motive).clone(), (*right).clone()))
            }
        }
    }

    fn expect_sort(&self, term: &Term, context: &[Term]) -> Result<u32, CheckError> {
        let actual = self.infer_in(term, context)?;
        match self.normalize(&actual) {
            Term::Prop => Ok(0),
            Term::Universe(level) => Ok(level),
            _ => Err(CheckError::ExpectedType { term: term.clone(), actual }),
        }
    }
}
