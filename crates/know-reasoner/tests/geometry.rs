//! Geometry acceptance suite — the required verdicts from the architecture
//! plan, evaluated by the Stage 2 Boolean reasoner.
//!
//!   Square SUBCLASS_OF Rectangle        → Entailed
//!   Square SUBCLASS_OF Polygon          → Entailed
//!   SATISFIABLE (Square AND Circle)     → Contradicted
//!   (Red AND Square) SUBCLASS_OF Large  → Unknown
//!   entity asserted Square and Circle   → Inconsistent

use know_ontology::ConceptExpr;
use know_reasoner::{BooleanReasoner, Proposition, Reasoner, ReasoningOutcome, Verdict};
use know_test_support::geometry::{geometry_module, geometry_with_impossible_entity, geometry_with_square_entity};

fn named(id: &str) -> ConceptExpr {
    ConceptExpr::named(id)
}

fn complete(outcome: ReasoningOutcome<Verdict>) -> Verdict {
    match outcome {
        ReasoningOutcome::Complete(v) => v,
        other => panic!("expected Complete verdict, got {other:?}"),
    }
}

fn query(reasoner: &BooleanReasoner, proposition: Proposition) -> Verdict {
    complete(reasoner.query(&proposition))
}

fn subclass(child: ConceptExpr, parent: ConceptExpr) -> Proposition {
    Proposition::SubclassOf { child, parent }
}

// ---------------------------------------------------------------------------
// Required verdicts
// ---------------------------------------------------------------------------

#[test]
fn square_is_subclass_of_rectangle() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let verdict = query(&reasoner, subclass(named("geometry::square"), named("geometry::rectangle")));
    let Verdict::Entailed(explanation) = verdict else {
        panic!("expected Entailed, got {}", verdict.kind());
    };
    // The proof must rest on the definition of square.
    assert!(
        explanation.supporting_axioms.iter().any(|a| a.0 == "definition:geometry::square"),
        "explanation should cite the square definition: {:?}",
        explanation.supporting_axioms
    );
}

#[test]
fn square_is_subclass_of_polygon() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let verdict = query(&reasoner, subclass(named("geometry::square"), named("geometry::polygon")));
    assert_eq!(verdict.kind(), "Entailed");
}

#[test]
fn square_and_circle_is_unsatisfiable() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let expr = ConceptExpr::and(vec![named("geometry::square"), named("geometry::circle")]);
    let verdict = query(&reasoner, Proposition::Satisfiable { class: expr.clone() });
    assert_eq!(verdict.kind(), "Contradicted");

    match reasoner.is_satisfiable(&expr) {
        ReasoningOutcome::Complete(satisfiable) => assert!(!satisfiable),
        other => panic!("expected Complete, got {other:?}"),
    }
}

#[test]
fn red_square_subclass_of_large_is_unknown() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let child = ConceptExpr::and(vec![named("geometry::red"), named("geometry::square")]);
    let verdict = query(&reasoner, subclass(child, named("geometry::large")));
    let Verdict::Unknown(explanation) = verdict else {
        panic!("expected Unknown, got {}", verdict.kind());
    };
    assert!(!explanation.missing.is_empty(), "Unknown must say what is missing");
}

#[test]
fn impossible_entity_makes_module_inconsistent() {
    let reasoner = BooleanReasoner::new(&geometry_with_impossible_entity());

    match reasoner.is_consistent() {
        ReasoningOutcome::Complete(consistent) => assert!(!consistent),
        other => panic!("expected Complete, got {other:?}"),
    }

    // Any query against an inconsistent KB reports the inconsistency.
    let verdict = query(&reasoner, subclass(named("geometry::square"), named("geometry::circle")));
    let Verdict::Inconsistent(report) = verdict else {
        panic!("expected Inconsistent, got {}", verdict.kind());
    };
    assert!(!report.conflicting_axioms.is_empty(), "must name conflicting axioms");
}

// ---------------------------------------------------------------------------
// Class membership under open-world semantics
// ---------------------------------------------------------------------------

#[test]
fn asserted_square_entity_is_entailed_polygon() {
    let reasoner = BooleanReasoner::new(&geometry_with_square_entity());
    let verdict = query(
        &reasoner,
        Proposition::ClassMembership {
            entity: know_core::EntityId("geometry::my_square".into()),
            class: named("geometry::polygon"),
        },
    );
    assert_eq!(verdict.kind(), "Entailed");
}

#[test]
fn asserted_square_entity_is_contradicted_circle() {
    let reasoner = BooleanReasoner::new(&geometry_with_square_entity());
    let verdict = query(
        &reasoner,
        Proposition::ClassMembership {
            entity: know_core::EntityId("geometry::my_square".into()),
            class: named("geometry::circle"),
        },
    );
    assert_eq!(verdict.kind(), "Contradicted");
}

#[test]
fn asserted_square_entity_redness_is_unknown() {
    // Open world: nothing says the square is red, nothing says it is not.
    let reasoner = BooleanReasoner::new(&geometry_with_square_entity());
    let verdict = query(
        &reasoner,
        Proposition::ClassMembership {
            entity: know_core::EntityId("geometry::my_square".into()),
            class: named("geometry::red"),
        },
    );
    assert_eq!(verdict.kind(), "Unknown");
}

// ---------------------------------------------------------------------------
// Other verdict forms
// ---------------------------------------------------------------------------

#[test]
fn square_is_equivalent_to_its_definition() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let verdict = query(
        &reasoner,
        Proposition::Equivalent {
            left: named("geometry::square"),
            right: ConceptExpr::and(vec![named("geometry::rectangle"), named("geometry::rhombus")]),
        },
    );
    assert_eq!(verdict.kind(), "Entailed");
}

#[test]
fn triangle_and_circle_are_disjoint_via_hierarchy() {
    // triangle ⊑ polygon, and polygon is disjoint with circle.
    let reasoner = BooleanReasoner::new(&geometry_module());
    let verdict =
        query(&reasoner, Proposition::Disjoint { left: named("geometry::triangle"), right: named("geometry::circle") });
    assert_eq!(verdict.kind(), "Entailed");
}

#[test]
fn rectangle_rhombus_disjointness_is_unknown() {
    // Nothing relates rectangle and rhombus beyond a shared parent, so
    // disjointness is open (squares exist, but the ontology doesn't say so).
    let reasoner = BooleanReasoner::new(&geometry_module());
    let verdict = query(
        &reasoner,
        Proposition::Disjoint { left: named("geometry::rectangle"), right: named("geometry::rhombus") },
    );
    assert_eq!(verdict.kind(), "Unknown");
}

#[test]
fn consistency_of_clean_module_is_entailed() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let verdict = query(&reasoner, Proposition::Consistent);
    assert_eq!(verdict.kind(), "Entailed");
}

#[test]
fn unknown_concept_in_query_is_ill_typed() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let verdict = query(&reasoner, subclass(named("geometry::pentagon"), named("geometry::polygon")));
    let Verdict::IllTyped(diags) = verdict else {
        panic!("expected IllTyped, got {}", verdict.kind());
    };
    assert!(diags[0].message.contains("geometry::pentagon"));
}

#[test]
fn relational_query_is_unsupported_not_unknown() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let child = ConceptExpr::Exists {
        relation: know_core::RelationId("geometry::has_side".into()),
        filler: Box::new(named("geometry::square")),
    };
    let outcome = reasoner.query(&subclass(child, named("geometry::polygon")));
    assert!(
        matches!(outcome, ReasoningOutcome::Unsupported(_)),
        "relational queries must be Unsupported, never approximated: {outcome:?}"
    );
}

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

#[test]
fn classify_square_finds_direct_parents() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let result = match reasoner.classify(&named("geometry::square")) {
        ReasoningOutcome::Complete(r) => r,
        other => panic!("expected Complete, got {other:?}"),
    };

    let supers: Vec<&str> = result.direct_superclasses.iter().map(|c| c.0.as_str()).collect();
    assert!(supers.contains(&"geometry::rectangle"), "direct supers: {supers:?}");
    assert!(supers.contains(&"geometry::rhombus"), "direct supers: {supers:?}");
    assert!(!supers.contains(&"geometry::polygon"), "polygon is an indirect superclass: {supers:?}");
    assert!(result.direct_subclasses.is_empty());
    assert!(result.equivalent_classes.is_empty());
}

#[test]
fn classify_quadrilateral_finds_children() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let result = match reasoner.classify(&named("geometry::quadrilateral")) {
        ReasoningOutcome::Complete(r) => r,
        other => panic!("expected Complete, got {other:?}"),
    };
    let subs: Vec<&str> = result.direct_subclasses.iter().map(|c| c.0.as_str()).collect();
    assert!(subs.contains(&"geometry::rectangle"), "direct subs: {subs:?}");
    assert!(subs.contains(&"geometry::rhombus"), "direct subs: {subs:?}");
    assert!(!subs.contains(&"geometry::square"), "square is indirect: {subs:?}");
}

// ---------------------------------------------------------------------------
// Semantic invariants (plan §14 property tests, fixed instances)
// ---------------------------------------------------------------------------

#[test]
fn entailed_subclass_implies_child_and_not_parent_unsatisfiable() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let child = named("geometry::square");
    let parent = named("geometry::polygon");

    let verdict = query(&reasoner, subclass(child.clone(), parent.clone()));
    assert_eq!(verdict.kind(), "Entailed");

    let residue = ConceptExpr::and(vec![child, ConceptExpr::not(parent)]);
    match reasoner.is_satisfiable(&residue) {
        ReasoningOutcome::Complete(satisfiable) => assert!(!satisfiable),
        other => panic!("expected Complete, got {other:?}"),
    }
}

#[test]
fn entailed_disjointness_implies_conjunction_unsatisfiable() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let conjunction = ConceptExpr::and(vec![named("geometry::polygon"), named("geometry::circle")]);
    match reasoner.is_satisfiable(&conjunction) {
        ReasoningOutcome::Complete(satisfiable) => assert!(!satisfiable),
        other => panic!("expected Complete, got {other:?}"),
    }
}

#[test]
fn equivalence_implies_subclass_both_directions() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let square = named("geometry::square");
    let def = ConceptExpr::and(vec![named("geometry::rectangle"), named("geometry::rhombus")]);

    assert_eq!(query(&reasoner, subclass(square.clone(), def.clone())).kind(), "Entailed");
    assert_eq!(query(&reasoner, subclass(def, square)).kind(), "Entailed");
}

#[test]
fn explanations_carry_premises_and_conclusion() {
    let reasoner = BooleanReasoner::new(&geometry_module());
    let proposition = subclass(named("geometry::square"), named("geometry::quadrilateral"));
    let Verdict::Entailed(explanation) = query(&reasoner, proposition) else {
        panic!("expected Entailed");
    };
    assert!(!explanation.supporting_axioms.is_empty());
    assert!(!explanation.steps.is_empty());
    assert!(!explanation.steps[0].premises.is_empty(), "premises should name the axioms used");
}
