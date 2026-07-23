//! Lexical resolution layer: maps natural-language word forms to canonical
//! concept IDs before any logical reasoning occurs.
//!
//! The central invariant (from the architecture plan, §2.2):
//!   Polysemy is NOT logical OR. "bank" does not become
//!   `FinancialBank OR RiverBank`. The lexical resolver selects (or reports
//!   ambiguity on) the intended concept before the proposition is formed.
//!
//! Confidence scores are allowed here; they must not bleed into concept
//! semantics or influence what the reasoner takes to be logically true.

use know_core::{ConceptId, LanguageId, Provenance};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Lexical form and bindings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartOfSpeech {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Preposition,
    // TODO: extend as the lexical model is specified.
}

/// One surface form of a word (e.g. "bank"), in a given language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexicalForm {
    pub text: String,
    pub language: LanguageId,
    pub part_of_speech: Option<PartOfSpeech>,
    /// All candidate concept bindings for this form, in no particular order.
    /// The resolver selects among these at query time.
    pub bindings: Vec<LexicalBinding>,
}

/// One candidate interpretation of a `LexicalForm`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexicalBinding {
    pub concept: ConceptId,
    /// Neighbouring concepts that, when present in the query context, raise
    /// confidence that this binding is the intended one.
    pub context_hints: Vec<ConceptId>,
    pub usage_examples: Vec<String>,
    pub provenance: Provenance,
}

/// A lexical module groups forms by language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexicalModule {
    pub language: LanguageId,
    pub forms: Vec<LexicalForm>,
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// The context available to the resolver when disambiguating a form.
#[derive(Debug, Clone, Default)]
pub struct ResolutionContext {
    /// Concepts already established in the current discourse or sentence.
    pub surrounding_concepts: Vec<ConceptId>,
    /// Explicit domain hint, if the caller can supply one.
    pub domain_hint: Option<ConceptId>,
    pub language: Option<LanguageId>,
}

/// Evidence for why a particular binding was selected.
#[derive(Debug, Clone)]
pub enum ResolutionEvidence {
    ContextHintMatch { hint: ConceptId },
    UsagePatternMatch,
    // TODO: add LlmInterpreted variant once the LLM integration layer is
    // specified. The LLM should return a ConceptId, not free text, so its
    // output is still subject to the same lexical validation.
    ExplicitDomainHint { domain: ConceptId },
}

/// One candidate returned by the resolver, with a confidence score.
///
/// Confidence is a local ranking aid, not a logical truth value.
#[derive(Debug, Clone)]
pub struct ResolutionCandidate {
    pub concept: ConceptId,
    /// [0.0, 1.0] — relative confidence among candidates for this form.
    pub confidence: f32,
    pub evidence: Vec<ResolutionEvidence>,
}

#[derive(Debug, Clone)]
pub enum ResolutionResult {
    /// Exactly one candidate clearly dominates.
    Resolved(ResolutionCandidate),
    /// Multiple candidates remain; the reasoner must not guess.
    Ambiguous(Vec<ResolutionCandidate>),
    /// No binding exists for this form in the loaded lexicon.
    NotFound,
}

/// Trait implemented by all resolution strategies.
///
/// TODO: implement a context-hint-based resolver as the first concrete
/// strategy. The plan specifies that the resolver may use surrounding words,
/// grammatical role, domain context, embeddings, LLM interpretation, and
/// previously resolved discourse entities — but the exact algorithm and
/// priority ordering are not yet specified.
pub trait Resolver {
    fn resolve(&self, text: &str, context: &ResolutionContext) -> ResolutionResult;
}

/// Placeholder resolver — always returns `NotFound`.
/// Exists so dependent crates compile while the real resolver is unbuilt.
pub struct UnimplementedResolver;

impl Resolver for UnimplementedResolver {
    fn resolve(&self, _text: &str, _context: &ResolutionContext) -> ResolutionResult {
        // TODO: implement context-hint-based resolution (see module doc).
        ResolutionResult::NotFound
    }
}
