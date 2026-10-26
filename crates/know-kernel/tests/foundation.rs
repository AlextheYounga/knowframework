#![expect(clippy::unwrap_used, reason = "failed fixture construction should fail the test immediately")]

use know_kernel::{CheckError, Term, Theory, TheoryError};

fn constant(name: &str) -> Term {
    Term::constant(name)
}

fn call(name: &str, arguments: impl IntoIterator<Item = Term>) -> Term {
    Term::apply_many(constant(name), arguments)
}

fn being() -> Term {
    constant("Being")
}

fn foundation() -> Theory {
    let mut theory = Theory::default();
    theory.declare_symbol("Being", Term::Universe(0)).unwrap();
    theory.declare_symbol("Characteristic", Term::Universe(0)).unwrap();
    theory.declare_symbol("identity", constant("Characteristic")).unwrap();
    theory
        .declare_symbol(
            "has_characteristic",
            Term::function(being(), Term::function(constant("Characteristic"), Term::Prop)),
        )
        .unwrap();
    theory.declare_symbol("precedes", Term::function(being(), Term::function(being(), Term::Prop))).unwrap();
    for entity in ["first", "second", "third"] {
        theory.declare_symbol(entity, being()).unwrap();
    }
    theory
}

#[test]
fn necessary_characteristic_applies_to_each_being() {
    let mut theory = foundation();
    let characteristic_of_bound_being = call("has_characteristic", [Term::bound(0), constant("identity")]);
    theory.assume("being_has_character", Term::pi(being(), characteristic_of_bound_being)).unwrap();

    let proposition = call("has_characteristic", [constant("first"), constant("identity")]);
    let proof = Term::apply(constant("being_has_character"), constant("first"));
    let theorem = theory.prove("first_has_character", proposition, proof).unwrap();

    assert_eq!(theorem.assumptions, ["being_has_character"]);
}

#[test]
fn identity_substitution_preserves_predication() {
    let mut theory = foundation();
    let equality = Term::equal(being(), constant("first"), constant("second"));
    theory.assume("first_is_second", equality).unwrap();
    theory
        .assume("first_has_character", call("has_characteristic", [constant("first"), constant("identity")]))
        .unwrap();

    let motive = Term::lambda(being(), call("has_characteristic", [Term::bound(0), constant("identity")]));
    let proof = Term::rewrite(constant("first_is_second"), motive, constant("first_has_character"));
    let theorem = theory
        .prove("second_has_character", call("has_characteristic", [constant("second"), constant("identity")]), proof)
        .unwrap();

    assert_eq!(theorem.assumptions, ["first_has_character", "first_is_second"]);
}

#[test]
fn declared_transitivity_licenses_its_relational_consequence() {
    let mut theory = foundation();
    let precedes = |left, right| call("precedes", [left, right]);

    let transitive_body = Term::implies(
        precedes(Term::bound(2), Term::bound(1)),
        Term::implies(precedes(Term::bound(1), Term::bound(0)), precedes(Term::bound(2), Term::bound(0))),
    );
    let transitive = Term::pi(being(), Term::pi(being(), Term::pi(being(), transitive_body)));
    theory.assume("precedes_is_transitive", transitive).unwrap();
    theory.assume("first_precedes_second", precedes(constant("first"), constant("second"))).unwrap();
    theory.assume("second_precedes_third", precedes(constant("second"), constant("third"))).unwrap();

    let instantiated_rule = Term::apply_many(
        constant("precedes_is_transitive"),
        [constant("first"), constant("second"), constant("third")],
    );
    let proof =
        Term::apply_many(instantiated_rule, [constant("first_precedes_second"), constant("second_precedes_third")]);
    let theorem = theory.prove("first_precedes_third", precedes(constant("first"), constant("third")), proof).unwrap();

    assert_eq!(theorem.assumptions, ["first_precedes_second", "precedes_is_transitive", "second_precedes_third"]);
}

#[test]
fn a_proposition_and_its_negation_produce_contradiction() {
    let mut theory = foundation();
    theory.declare_symbol("False", Term::Prop).unwrap();
    let proposition = call("has_characteristic", [constant("first"), constant("identity")]);
    theory.assume("positive", proposition.clone()).unwrap();
    theory.assume("negative", Term::implies(proposition, constant("False"))).unwrap();

    let proof = Term::apply(constant("negative"), constant("positive"));
    let theorem = theory.prove("contradiction", constant("False"), proof).unwrap();

    assert_eq!(theorem.assumptions, ["negative", "positive"]);
}

#[test]
fn universal_classification_does_not_prove_existence() {
    let mut theory = foundation();
    theory.declare_symbol("ExistsBeing", Term::function(Term::function(being(), Term::Prop), Term::Prop)).unwrap();
    let characteristic = Term::lambda(being(), call("has_characteristic", [Term::bound(0), constant("identity")]));
    let universal = Term::pi(being(), call("has_characteristic", [Term::bound(0), constant("identity")]));
    theory.assume("every_being_has_character", universal).unwrap();

    let error = theory
        .prove(
            "a_being_exists",
            Term::apply(constant("ExistsBeing"), characteristic),
            constant("every_being_has_character"),
        )
        .unwrap_err();

    assert!(matches!(error, TheoryError::Check(CheckError::TypeMismatch { .. })));
}

#[test]
fn implication_and_universal_proofs_are_lambda_terms() {
    let mut theory = foundation();
    let proposition = call("has_characteristic", [constant("first"), constant("identity")]);
    let identity_implication = Term::implies(proposition.clone(), proposition);
    theory
        .prove(
            "identity_implication",
            identity_implication,
            Term::lambda(call("has_characteristic", [constant("first"), constant("identity")]), Term::bound(0)),
        )
        .unwrap();

    let self_identity = Term::pi(being(), Term::equal(being(), Term::bound(0), Term::bound(0)));
    theory.prove("every_being_is_itself", self_identity, Term::lambda(being(), Term::refl(Term::bound(0)))).unwrap();
}

#[test]
fn conjunction_is_definable_and_checkable_in_the_calculus() {
    let mut theory = foundation();
    let left = call("has_characteristic", [constant("first"), constant("identity")]);
    let right = call("has_characteristic", [constant("second"), constant("identity")]);
    theory.assume("left", left.clone()).unwrap();
    theory.assume("right", right.clone()).unwrap();

    // The Church encoding keeps conjunction in the library rather than adding
    // a special kernel branch: forall result: Prop. (left -> right -> result) -> result.
    let combine = Term::implies(left, Term::implies(right, Term::bound(0)));
    let conjunction = Term::pi(Term::Prop, Term::implies(combine.clone(), Term::bound(0)));
    let proof = Term::lambda(
        Term::Prop,
        Term::lambda(combine, Term::apply_many(Term::bound(0), [constant("left"), constant("right")])),
    );

    let theorem = theory.prove("both", conjunction, proof).unwrap();
    assert_eq!(theorem.assumptions, ["left", "right"]);
}

#[test]
fn classical_principles_are_explicit_assumptions() {
    let mut theory = foundation();
    theory.declare_symbol("Or", Term::function(Term::Prop, Term::function(Term::Prop, Term::Prop))).unwrap();
    theory.declare_symbol("False", Term::Prop).unwrap();
    let excluded_middle =
        Term::pi(Term::Prop, call("Or", [Term::bound(0), Term::implies(Term::bound(0), constant("False"))]));

    theory.assume("excluded_middle", excluded_middle).unwrap();

    assert_eq!(theory.declaration("excluded_middle").unwrap().value, None);
}

#[test]
fn malformed_and_unsupported_proofs_are_rejected() {
    let mut theory = foundation();
    theory.declare_symbol("Other", Term::Universe(0)).unwrap();
    theory.declare_symbol("other", constant("Other")).unwrap();

    let wrong_argument = call("has_characteristic", [constant("other"), constant("identity")]);
    assert!(matches!(theory.checker().infer(&wrong_argument), Err(CheckError::TypeMismatch { .. })));
    assert!(matches!(theory.checker().infer(&Term::bound(0)), Err(CheckError::EscapedVariable { .. })));
    assert!(matches!(
        theory.declare_symbol("not_a_type", constant("first")),
        Err(TheoryError::Check(CheckError::ExpectedType { .. }))
    ));

    let circular = theory.define("loop", being(), constant("loop"));
    assert_eq!(circular, Err(TheoryError::CircularDefinition("loop".to_string())));

    assert_eq!(
        theory.declare_symbol(
            "hidden_assumption",
            call("has_characteristic", [constant("first"), constant("identity")]),
        ),
        Err(TheoryError::ProofRequiresAssumption("hidden_assumption".to_string()))
    );

    let unsupported = theory.prove(
        "unsupported_conclusion",
        call("has_characteristic", [constant("first"), constant("identity")]),
        constant("first"),
    );
    assert!(matches!(unsupported, Err(TheoryError::Check(CheckError::TypeMismatch { .. }))));
}
