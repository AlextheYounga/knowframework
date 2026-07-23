use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use know_admission::{KnowledgeProposal, Pipeline};
use know_lexicon::{ContextResolver, ResolutionContext, ResolutionResult, Resolver};
use know_ontology::{
    ConceptExpr, ConceptExprSource, KnowledgeModule, KnowledgeModuleSource, compile,
};
use know_owl::export_owl_functional;
use know_reasoner::{
    BooleanReasoner, Explanation, Proposition, Reasoner, ReasoningOutcome, Verdict,
};

#[derive(Parser)]
#[command(name = "know", about = "Know knowledge framework")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and validate a knowledge module file.
    Check { path: PathBuf },

    /// Parse, validate, and reserialize a module in canonical form.
    Normalize { path: PathBuf },

    /// Evaluate a query against a module.
    ///
    /// Query forms:
    ///   consistent
    ///   satisfiable <expr>
    ///   subclass <expr> <expr>
    ///   equivalent <expr> <expr>
    ///   disjoint <expr> <expr>
    ///   member <entity-id> <expr>
    ///
    /// An <expr> is a bare concept ID, or a RON concept expression such as
    /// 'And([Named("geometry::red"), Named("geometry::square")])'.
    Reason {
        module: PathBuf,
        query: Vec<String>,
    },

    /// Classify a concept within a module (direct super/sub/equivalent classes).
    Classify {
        module: PathBuf,
        concept: String,
    },

    /// Like `reason`, but prints the full explanation artifact.
    Explain {
        module: PathBuf,
        query: Vec<String>,
    },

    /// Resolve a word to candidate canonical concepts using a lexicon module.
    Resolve {
        lexicon: PathBuf,
        text: String,
        /// Comma-separated concept IDs already established in context.
        #[arg(long)]
        context: Option<String>,
        /// Explicit domain-hint concept ID.
        #[arg(long)]
        domain: Option<String>,
    },

    /// Run a knowledge proposal (RON `KnowledgeProposal`) through the
    /// admission pipeline against a base module.
    ///
    /// TODO: load regression checks from a per-module manifest once their
    /// storage format is specified; currently only the always-on stages run.
    Admit {
        module: PathBuf,
        proposal: PathBuf,
    },

    /// Export a module to OWL 2 Functional Syntax.
    ExportOwl { module: PathBuf },

    /// Import an OWL ontology as a Know source module.
    ImportOwl { ontology: PathBuf },

    /// Compare this module's reasoning results against a reference OWL reasoner.
    ///
    /// TODO: implement once OWL export and differential testing are in place.
    DiffReasoner { module: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Check { path } => {
            load_and_compile(&path)?;
            println!("ok");
            Ok(())
        }

        Command::Normalize { path } => {
            let source = load_module(&path)?;
            compile_source(source.clone())?;
            let canonical = source.to_ron().map_err(|e| e.to_string())?;
            println!("{canonical}");
            Ok(())
        }

        Command::Reason { module, query } => {
            let module = load_and_compile(&module)?;
            let proposition = parse_query(&query)?;
            let reasoner = BooleanReasoner::new(&module);
            print_verdict(reasoner.query(&proposition), false)
        }

        Command::Explain { module, query } => {
            let module = load_and_compile(&module)?;
            let proposition = parse_query(&query)?;
            let reasoner = BooleanReasoner::new(&module);
            print_verdict(reasoner.query(&proposition), true)
        }

        Command::Classify { module, concept } => {
            let module = load_and_compile(&module)?;
            let reasoner = BooleanReasoner::new(&module);
            match reasoner.classify(&ConceptExpr::named(&concept)) {
                ReasoningOutcome::Complete(result) => {
                    print_ids("direct superclasses", result.direct_superclasses.iter().map(|c| &c.0));
                    print_ids("direct subclasses", result.direct_subclasses.iter().map(|c| &c.0));
                    print_ids("equivalent classes", result.equivalent_classes.iter().map(|c| &c.0));
                    Ok(())
                }
                other => Err(format!("classification did not complete: {other:?}")),
            }
        }

        Command::Resolve { lexicon, text, context, domain } => {
            let content = read(&lexicon)?;
            let lexicon = know_lexicon::LexicalModule::from_ron(&content)
                .map_err(|e| format!("{e}"))?;
            let ctx = ResolutionContext {
                surrounding_concepts: context
                    .map(|c| c.split(',').map(|s| know_core::ConceptId(s.trim().to_string())).collect())
                    .unwrap_or_default(),
                domain_hint: domain.map(know_core::ConceptId),
                language: None,
            };
            match ContextResolver::new(lexicon).resolve(&text, &ctx) {
                ResolutionResult::Resolved(c) => {
                    println!("resolved: {} (confidence {:.2})", c.concept.0, c.confidence);
                    Ok(())
                }
                ResolutionResult::Ambiguous(candidates) => {
                    println!("ambiguous among {} candidates:", candidates.len());
                    for c in candidates {
                        println!("  {} (confidence {:.2})", c.concept.0, c.confidence);
                    }
                    Ok(())
                }
                ResolutionResult::NotFound => Err(format!("no lexical binding for '{text}'")),
            }
        }

        Command::Admit { module, proposal } => {
            let base = load_module(&module)?;
            let proposal = KnowledgeProposal::from_ron(&read(&proposal)?)
                .map_err(|e| format!("proposal: {e}"))?;
            let record = Pipeline::new(base).admit(proposal);

            for result in &record.validation_results {
                let status = if result.passed { "pass" } else { "FAIL" };
                println!("{:?}: {status}", result.stage);
                for d in &result.diagnostics {
                    println!("  {d}");
                }
            }
            println!("decision: {}", describe_decision(&record.decision));
            Ok(())
        }

        Command::ExportOwl { module } => {
            let m = load_and_compile(&module)?;
            let owl = export_owl_functional(&m)?;
            println!("{owl}");
            Ok(())
        }

        Command::ImportOwl { ontology } => {
            let _ = ontology;
            Err("import-owl: not yet implemented (Phase 5)".to_string())
        }

        Command::DiffReasoner { module } => {
            let _ = module;
            Err("diff-reasoner: not yet implemented (Phase 5)".to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Query parsing
// ---------------------------------------------------------------------------

fn parse_query(words: &[String]) -> Result<Proposition, String> {
    let usage = "expected: consistent | satisfiable <expr> | subclass <expr> <expr> | \
                 equivalent <expr> <expr> | disjoint <expr> <expr> | member <entity> <expr>";

    let (head, rest) = words.split_first().ok_or(usage)?;
    let arity = |n: usize| -> Result<(), String> {
        if rest.len() == n {
            Ok(())
        } else {
            Err(format!("'{head}' takes {n} argument(s); {usage}"))
        }
    };

    match head.as_str() {
        "consistent" => {
            arity(0)?;
            Ok(Proposition::Consistent)
        }
        "satisfiable" => {
            arity(1)?;
            Ok(Proposition::Satisfiable { class: parse_expr(&rest[0])? })
        }
        "subclass" => {
            arity(2)?;
            Ok(Proposition::SubclassOf {
                child: parse_expr(&rest[0])?,
                parent: parse_expr(&rest[1])?,
            })
        }
        "equivalent" => {
            arity(2)?;
            Ok(Proposition::Equivalent {
                left: parse_expr(&rest[0])?,
                right: parse_expr(&rest[1])?,
            })
        }
        "disjoint" => {
            arity(2)?;
            Ok(Proposition::Disjoint {
                left: parse_expr(&rest[0])?,
                right: parse_expr(&rest[1])?,
            })
        }
        "member" => {
            arity(2)?;
            Ok(Proposition::ClassMembership {
                entity: know_core::EntityId(rest[0].clone()),
                class: parse_expr(&rest[1])?,
            })
        }
        other => Err(format!("unknown query form '{other}'; {usage}")),
    }
}

/// A bare concept ID, or a RON `ConceptExprSource`.
fn parse_expr(input: &str) -> Result<ConceptExpr, String> {
    let source = if input.contains('(') {
        ron::from_str::<ConceptExprSource>(input)
            .map_err(|e| format!("bad concept expression '{input}': {e}"))?
    } else {
        ConceptExprSource::Named(input.to_string())
    };
    Ok(expr_from_source(source))
}

/// Syntactic source→IR conversion. Vocabulary checking happens in the
/// reasoner, which reports unknown names as IllTyped.
fn expr_from_source(source: ConceptExprSource) -> ConceptExpr {
    match source {
        ConceptExprSource::Named(name) => ConceptExpr::Named(know_core::ConceptId(name)),
        ConceptExprSource::And(parts) => {
            ConceptExpr::And(parts.into_iter().map(expr_from_source).collect())
        }
        ConceptExprSource::Or(parts) => {
            ConceptExpr::Or(parts.into_iter().map(expr_from_source).collect())
        }
        ConceptExprSource::Not(inner) => ConceptExpr::Not(Box::new(expr_from_source(*inner))),
        ConceptExprSource::Exists { relation, filler } => ConceptExpr::Exists {
            relation: know_core::RelationId(relation),
            filler: Box::new(expr_from_source(*filler)),
        },
        ConceptExprSource::ForAll { relation, filler } => ConceptExpr::ForAll {
            relation: know_core::RelationId(relation),
            filler: Box::new(expr_from_source(*filler)),
        },
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

fn print_verdict(outcome: ReasoningOutcome<Verdict>, verbose: bool) -> Result<(), String> {
    match outcome {
        ReasoningOutcome::Complete(verdict) => {
            println!("{}", verdict.kind());
            match &verdict {
                Verdict::Entailed(e) | Verdict::Contradicted(e) => print_explanation(e, verbose),
                Verdict::Unknown(u) => {
                    for m in &u.missing {
                        println!("  {m}");
                    }
                }
                Verdict::IllTyped(diags) => {
                    for d in diags {
                        println!("  {d}");
                    }
                }
                Verdict::Inconsistent(report) => {
                    if let Some(explanation) = &report.explanation {
                        println!("  {explanation}");
                    }
                    for a in &report.conflicting_axioms {
                        println!("  conflicting axiom: {}", a.0);
                    }
                }
                Verdict::Ambiguous(report) => {
                    for c in &report.candidates {
                        println!("  candidate: {}", c.0);
                    }
                }
            }
            Ok(())
        }
        ReasoningOutcome::Unsupported(u) => {
            let advice = u.advice.map(|a| format!(" ({a})")).unwrap_or_default();
            Err(format!("unsupported: {}{advice}", u.feature))
        }
        ReasoningOutcome::ResourceLimit(limit) => Err(format!("resource limit: {limit:?}")),
        ReasoningOutcome::InternalError(message) => Err(format!("internal error: {message}")),
    }
}

fn print_explanation(explanation: &Explanation, verbose: bool) {
    if let Some(notes) = &explanation.notes {
        println!("  {notes}");
    }
    if !verbose {
        return;
    }
    for step in &explanation.steps {
        println!("  by {:?}:", step.rule);
        for premise in &step.premises {
            println!("    given {premise}");
        }
        println!("    conclude {}", step.conclusion);
    }
    for axiom in &explanation.supporting_axioms {
        println!("  supporting axiom: {}", axiom.0);
    }
}

fn print_ids<'a>(label: &str, ids: impl Iterator<Item = &'a String>) {
    let list: Vec<&str> = ids.map(|s| s.as_str()).collect();
    println!("{label}: {}", if list.is_empty() { "(none)".to_string() } else { list.join(", ") });
}

fn describe_decision(decision: &know_admission::AdmissionDecision) -> String {
    use know_admission::AdmissionDecision as D;
    match decision {
        D::Accepted => "accepted".to_string(),
        D::AcceptedWithWarnings(w) => format!("accepted with {} warning(s)", w.len()),
        D::Rejected(e) => format!("rejected ({} error(s))", e.len()),
        D::DeferredForAmbiguity(_) => "deferred: ambiguous".to_string(),
        D::DeferredForGrounding(_) => "deferred: grounding".to_string(),
        D::ConflictsWithExistingKnowledge(_) => "conflicts with existing knowledge".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn read(path: &PathBuf) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn load_module(path: &PathBuf) -> Result<KnowledgeModuleSource, String> {
    KnowledgeModuleSource::from_ron(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn compile_source(source: KnowledgeModuleSource) -> Result<KnowledgeModule, String> {
    compile::compile(source).map_err(|e| {
        let mut message = e.to_string();
        for d in e.diagnostics() {
            message.push_str(&format!("\n  {d}"));
        }
        message
    })
}

fn load_and_compile(path: &PathBuf) -> Result<KnowledgeModule, String> {
    compile_source(load_module(path)?)
}
