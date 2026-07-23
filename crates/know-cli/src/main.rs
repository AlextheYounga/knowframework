use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use know_ontology::{KnowledgeModuleSource, compile};
use know_reasoner::UnimplementedReasoner;
use know_owl::export_owl_functional;

#[derive(Parser)]
#[command(name = "know", about = "Know knowledge framework")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and validate a knowledge module directory or file.
    Check { path: PathBuf },

    /// Parse, validate, and reserialize a module in canonical form.
    Normalize { path: PathBuf },

    /// Evaluate a query against a module.
    ///
    /// Query format is not yet specified; this stub accepts a free-form string.
    /// TODO: define query syntax once the reasoner interface is complete.
    Reason {
        module: PathBuf,
        query: String,
    },

    /// Classify a concept within a module (show superclasses / subclasses).
    Classify {
        module: PathBuf,
        concept: String,
    },

    /// Explain the reasoning behind a query result.
    Explain {
        module: PathBuf,
        query: String,
    },

    /// Resolve a word or phrase to candidate canonical concepts.
    ///
    /// TODO: wire up `know-lexicon` resolver once it is implemented.
    Resolve {
        module: PathBuf,
        text: String,
    },

    /// Run a knowledge proposal through the admission pipeline.
    ///
    /// TODO: wire up `know-admission` once the pipeline is implemented.
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
            let source = load_module(&path)?;
            compile::compile(source).map_err(|e| e.to_string())?;
            println!("ok");
            Ok(())
        }

        Command::Normalize { path } => {
            let source = load_module(&path)?;
            // Compile to validate, then re-serialize from source.
            compile::compile(source.clone()).map_err(|e| e.to_string())?;
            let canonical = source.to_ron().map_err(|e| e.to_string())?;
            println!("{canonical}");
            Ok(())
        }

        Command::Reason { module, query } => {
            let _module = load_and_compile(&module)?;
            let _ = query;
            // TODO: parse `query` into a `Proposition` and call reasoner.
            // The query syntax and Proposition parser are not yet specified.
            Err("reason: query syntax not yet implemented".to_string())
        }

        Command::Classify { module, concept } => {
            let module = load_and_compile(&module)?;
            let expr = know_ontology::ConceptExpr::named(&concept);
            match UnimplementedReasoner::new(&module).classify_stub(&expr) {
                Some(r) => println!("{r:?}"),
                None => println!("classification not yet implemented"),
            }
            Ok(())
        }

        Command::Explain { module, query } => {
            let _ = (module, query);
            Err("explain: not yet implemented".to_string())
        }

        Command::Resolve { module, text } => {
            let _ = (module, text);
            Err("resolve: lexical resolver not yet implemented".to_string())
        }

        Command::Admit { module, proposal } => {
            let _ = (module, proposal);
            Err("admit: admission pipeline not yet implemented".to_string())
        }

        Command::ExportOwl { module } => {
            let m = load_and_compile(&module)?;
            match export_owl_functional(&m) {
                Ok(owl) => { println!("{owl}"); Ok(()) }
                Err(e) => Err(e),
            }
        }

        Command::ImportOwl { ontology } => {
            let _ = ontology;
            Err("import-owl: not yet implemented".to_string())
        }

        Command::DiffReasoner { module } => {
            let _ = module;
            Err("diff-reasoner: not yet implemented".to_string())
        }
    }
}

fn load_module(path: &PathBuf) -> Result<KnowledgeModuleSource, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    KnowledgeModuleSource::from_ron(&text)
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn load_and_compile(path: &PathBuf) -> Result<know_ontology::KnowledgeModule, String> {
    let source = load_module(path)?;
    compile::compile(source).map_err(|e| e.to_string())
}

// Temporary extension to make `classify` compile until the Reasoner trait
// impl is wired into the CLI properly.
trait ClassifyStub {
    fn classify_stub(&self, expr: &know_ontology::ConceptExpr) -> Option<()>;
}

impl ClassifyStub for UnimplementedReasoner {
    fn classify_stub(&self, _expr: &know_ontology::ConceptExpr) -> Option<()> {
        None
    }
}
