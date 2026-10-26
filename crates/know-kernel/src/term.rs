use std::collections::HashSet;

/// A term in the kernel's dependently typed lambda calculus.
///
/// Bound variables use de Bruijn indices. Index zero names the innermost
/// binder, which makes alpha-equivalence structural and substitution
/// independent of source-level variable names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    /// The impredicative universe of propositions.
    Prop,
    /// A predicative universe of data types.
    Universe(u32),
    Bound(usize),
    Constant(String),
    Pi {
        parameter_type: Box<Term>,
        body: Box<Term>,
    },
    Lambda {
        parameter_type: Box<Term>,
        body: Box<Term>,
    },
    Apply {
        function: Box<Term>,
        argument: Box<Term>,
    },
    Equal {
        ty: Box<Term>,
        left: Box<Term>,
        right: Box<Term>,
    },
    Refl(Box<Term>),
    Rewrite {
        equality: Box<Term>,
        motive: Box<Term>,
        proof: Box<Term>,
    },
}

impl Term {
    #[must_use]
    pub fn constant(name: impl Into<String>) -> Self {
        Self::Constant(name.into())
    }

    #[must_use]
    pub const fn bound(index: usize) -> Self {
        Self::Bound(index)
    }

    #[must_use]
    pub fn pi(parameter_type: Self, body: Self) -> Self {
        Self::Pi { parameter_type: Box::new(parameter_type), body: Box::new(body) }
    }

    /// Construct a function type whose result does not depend on its argument.
    #[must_use]
    #[expect(clippy::needless_pass_by_value, reason = "term constructors consistently take ownership")]
    pub fn function(parameter_type: Self, result_type: Self) -> Self {
        Self::pi(parameter_type, result_type.shift(1, 0))
    }

    /// Logical implication under the propositions-as-types interpretation.
    #[must_use]
    pub fn implies(premise: Self, conclusion: Self) -> Self {
        Self::function(premise, conclusion)
    }

    #[must_use]
    pub fn lambda(parameter_type: Self, body: Self) -> Self {
        Self::Lambda { parameter_type: Box::new(parameter_type), body: Box::new(body) }
    }

    #[must_use]
    pub fn apply(function: Self, argument: Self) -> Self {
        Self::Apply { function: Box::new(function), argument: Box::new(argument) }
    }

    #[must_use]
    pub fn apply_many(function: Self, arguments: impl IntoIterator<Item = Self>) -> Self {
        arguments.into_iter().fold(function, Self::apply)
    }

    #[must_use]
    pub fn equal(ty: Self, left: Self, right: Self) -> Self {
        Self::Equal { ty: Box::new(ty), left: Box::new(left), right: Box::new(right) }
    }

    #[must_use]
    pub fn refl(value: Self) -> Self {
        Self::Refl(Box::new(value))
    }

    /// Eliminate equality by replacing equals in a proposition-valued motive.
    #[must_use]
    pub fn rewrite(equality: Self, motive: Self, proof: Self) -> Self {
        Self::Rewrite { equality: Box::new(equality), motive: Box::new(motive), proof: Box::new(proof) }
    }

    /// Shift free indices at or above `cutoff` by `amount`.
    #[must_use]
    pub(crate) fn shift(&self, amount: isize, cutoff: usize) -> Self {
        fn shifted(index: usize, amount: isize) -> usize {
            if amount.is_negative() {
                index.saturating_sub(amount.unsigned_abs())
            } else {
                index.saturating_add(amount.unsigned_abs())
            }
        }

        match self {
            Self::Prop => Self::Prop,
            Self::Universe(level) => Self::Universe(*level),
            Self::Bound(index) if *index >= cutoff => Self::Bound(shifted(*index, amount)),
            Self::Bound(index) => Self::Bound(*index),
            Self::Constant(name) => Self::Constant(name.clone()),
            Self::Pi { parameter_type, body } => {
                Self::pi(parameter_type.shift(amount, cutoff), body.shift(amount, cutoff + 1))
            }
            Self::Lambda { parameter_type, body } => {
                Self::lambda(parameter_type.shift(amount, cutoff), body.shift(amount, cutoff + 1))
            }
            Self::Apply { function, argument } => {
                Self::apply(function.shift(amount, cutoff), argument.shift(amount, cutoff))
            }
            Self::Equal { ty, left, right } => {
                Self::equal(ty.shift(amount, cutoff), left.shift(amount, cutoff), right.shift(amount, cutoff))
            }
            Self::Refl(value) => Self::refl(value.shift(amount, cutoff)),
            Self::Rewrite { equality, motive, proof } => {
                Self::rewrite(equality.shift(amount, cutoff), motive.shift(amount, cutoff), proof.shift(amount, cutoff))
            }
        }
    }

    /// Replace the variable at `index` with `replacement`.
    #[must_use]
    fn substitute(&self, index: usize, replacement: &Self) -> Self {
        match self {
            Self::Prop => Self::Prop,
            Self::Universe(level) => Self::Universe(*level),
            Self::Bound(current) if *current == index => replacement.clone(),
            Self::Bound(current) => Self::Bound(*current),
            Self::Constant(name) => Self::Constant(name.clone()),
            Self::Pi { parameter_type, body } => Self::pi(
                parameter_type.substitute(index, replacement),
                body.substitute(index + 1, &replacement.shift(1, 0)),
            ),
            Self::Lambda { parameter_type, body } => Self::lambda(
                parameter_type.substitute(index, replacement),
                body.substitute(index + 1, &replacement.shift(1, 0)),
            ),
            Self::Apply { function, argument } => {
                Self::apply(function.substitute(index, replacement), argument.substitute(index, replacement))
            }
            Self::Equal { ty, left, right } => Self::equal(
                ty.substitute(index, replacement),
                left.substitute(index, replacement),
                right.substitute(index, replacement),
            ),
            Self::Refl(value) => Self::refl(value.substitute(index, replacement)),
            Self::Rewrite { equality, motive, proof } => Self::rewrite(
                equality.substitute(index, replacement),
                motive.substitute(index, replacement),
                proof.substitute(index, replacement),
            ),
        }
    }

    /// Instantiate the outermost binder and remove it from the result.
    #[must_use]
    pub(crate) fn instantiate(&self, argument: &Self) -> Self {
        self.substitute(0, &argument.shift(1, 0)).shift(-1, 0)
    }

    pub(crate) fn constants(&self, output: &mut HashSet<String>) {
        match self {
            Self::Constant(name) => {
                output.insert(name.clone());
            }
            Self::Pi { parameter_type, body } | Self::Lambda { parameter_type, body } => {
                parameter_type.constants(output);
                body.constants(output);
            }
            Self::Apply { function, argument } => {
                function.constants(output);
                argument.constants(output);
            }
            Self::Equal { ty, left, right } => {
                ty.constants(output);
                left.constants(output);
                right.constants(output);
            }
            Self::Refl(value) => value.constants(output),
            Self::Rewrite { equality, motive, proof } => {
                equality.constants(output);
                motive.constants(output);
                proof.constants(output);
            }
            Self::Prop | Self::Universe(_) | Self::Bound(_) => {}
        }
    }
}
