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

impl LexicalModule {
    pub fn from_ron(input: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(input)
    }

    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }
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
pub trait Resolver {
    fn resolve(&self, text: &str, context: &ResolutionContext) -> ResolutionResult;
}

// ---------------------------------------------------------------------------
// Context-hint resolver
//
// The first concrete strategy from the plan's list (surrounding words,
// domain context, ...): scores each candidate binding by how much of the
// query context supports it. Embedding- and LLM-assisted candidate
// generation are later strategies behind the same trait.
// TODO: add an LLM-assisted resolver; it must return a ConceptId, not free
// text, so its output stays subject to the same validation.
// ---------------------------------------------------------------------------

/// Points per matched context hint / per explicit domain-hint match. A domain
/// hint is an explicit statement of intent from the caller, so it outweighs
/// any single incidental co-occurrence.
const HINT_POINTS: u32 = 1;
const DOMAIN_POINTS: u32 = 2;

pub struct ContextResolver {
    lexicon: LexicalModule,
}

impl ContextResolver {
    pub fn new(lexicon: LexicalModule) -> Self {
        Self { lexicon }
    }

    fn score(binding: &LexicalBinding, context: &ResolutionContext) -> (u32, Vec<ResolutionEvidence>) {
        let mut points = 0;
        let mut evidence = vec![];
        for hint in &binding.context_hints {
            if context.surrounding_concepts.contains(hint) {
                points += HINT_POINTS;
                evidence.push(ResolutionEvidence::ContextHintMatch { hint: hint.clone() });
            }
            if context.domain_hint.as_ref() == Some(hint) {
                points += DOMAIN_POINTS;
                evidence.push(ResolutionEvidence::ExplicitDomainHint { domain: hint.clone() });
            }
        }
        (points, evidence)
    }
}

impl Resolver for ContextResolver {
    fn resolve(&self, text: &str, context: &ResolutionContext) -> ResolutionResult {
        if let Some(lang) = &context.language
            && lang != &self.lexicon.language
        {
            return ResolutionResult::NotFound;
        }

        let scored: Vec<(u32, ResolutionCandidate)> = self
            .lexicon
            .forms
            .iter()
            .filter(|form| form.text.eq_ignore_ascii_case(text))
            .flat_map(|form| &form.bindings)
            .map(|binding| {
                let (points, evidence) = Self::score(binding, context);
                (points, ResolutionCandidate {
                    concept: binding.concept.clone(),
                    confidence: 0.0, // filled in below once totals are known
                    evidence,
                })
            })
            .collect();

        if scored.is_empty() {
            return ResolutionResult::NotFound;
        }

        let total: u32 = scored.iter().map(|(p, _)| *p).sum();
        let count = scored.len();
        let mut candidates: Vec<(u32, ResolutionCandidate)> = scored
            .into_iter()
            .map(|(points, mut candidate)| {
                // With no evidence anywhere, all candidates share confidence
                // equally; otherwise confidence is the candidate's share of
                // the total evidence. A ranking aid only — never truth.
                candidate.confidence = if total == 0 {
                    1.0 / count as f32
                } else {
                    points as f32 / total as f32
                };
                (points, candidate)
            })
            .collect();
        candidates.sort_by_key(|(points, _)| std::cmp::Reverse(*points));

        let sole_candidate = candidates.len() == 1;
        let strictly_dominant =
            candidates.len() > 1 && candidates[0].0 > candidates[1].0 && candidates[0].0 > 0;

        if sole_candidate || strictly_dominant {
            ResolutionResult::Resolved(candidates.remove(0).1)
        } else {
            // Ties and zero-evidence multi-candidate cases stay ambiguous:
            // the reasoner never guesses among senses.
            ResolutionResult::Ambiguous(candidates.into_iter().map(|(_, c)| c).collect())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(concept: &str, hints: &[&str]) -> LexicalBinding {
        LexicalBinding {
            concept: ConceptId(concept.to_string()),
            context_hints: hints.iter().map(|h| ConceptId(h.to_string())).collect(),
            usage_examples: vec![],
            provenance: Provenance::default(),
        }
    }

    /// The canonical polysemy fixture from the plan: "bank".
    fn bank_lexicon() -> ContextResolver {
        ContextResolver::new(LexicalModule {
            language: LanguageId::english(),
            forms: vec![LexicalForm {
                text: "bank".to_string(),
                language: LanguageId::english(),
                part_of_speech: Some(PartOfSpeech::Noun),
                bindings: vec![
                    binding(
                        "finance::bank",
                        &["finance::loan", "finance::deposit", "finance::account"],
                    ),
                    binding(
                        "geography::river_bank",
                        &["geography::river", "geography::shore"],
                    ),
                ],
            }],
        })
    }

    fn context(surrounding: &[&str]) -> ResolutionContext {
        ResolutionContext {
            surrounding_concepts: surrounding.iter().map(|c| ConceptId(c.to_string())).collect(),
            domain_hint: None,
            language: None,
        }
    }

    #[test]
    fn polysemy_without_context_is_ambiguous_not_a_union() {
        let result = bank_lexicon().resolve("bank", &ResolutionContext::default());
        let ResolutionResult::Ambiguous(candidates) = result else {
            panic!("expected Ambiguous");
        };
        // Distinct candidate concept IDs — never a merged/union concept.
        let ids: Vec<&str> = candidates.iter().map(|c| c.concept.0.as_str()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"finance::bank"));
        assert!(ids.contains(&"geography::river_bank"));
    }

    #[test]
    fn finance_context_resolves_to_finance_bank() {
        let result = bank_lexicon().resolve("bank", &context(&["finance::loan"]));
        let ResolutionResult::Resolved(candidate) = result else {
            panic!("expected Resolved");
        };
        assert_eq!(candidate.concept.0, "finance::bank");
        assert!(matches!(candidate.evidence[0], ResolutionEvidence::ContextHintMatch { .. }));
    }

    #[test]
    fn geography_context_resolves_to_river_bank() {
        let result = bank_lexicon().resolve("bank", &context(&["geography::river"]));
        let ResolutionResult::Resolved(candidate) = result else {
            panic!("expected Resolved");
        };
        assert_eq!(candidate.concept.0, "geography::river_bank");
    }

    #[test]
    fn conflicting_context_with_equal_evidence_stays_ambiguous() {
        let result = bank_lexicon().resolve("bank", &context(&["finance::loan", "geography::river"]));
        assert!(matches!(result, ResolutionResult::Ambiguous(_)));
    }

    #[test]
    fn domain_hint_outweighs_single_context_hint() {
        let ctx = ResolutionContext {
            surrounding_concepts: vec![ConceptId("finance::loan".to_string())],
            domain_hint: Some(ConceptId("geography::river".to_string())),
            language: None,
        };
        let ResolutionResult::Resolved(candidate) = bank_lexicon().resolve("bank", &ctx) else {
            panic!("expected Resolved");
        };
        assert_eq!(candidate.concept.0, "geography::river_bank");
    }

    #[test]
    fn unknown_word_is_not_found() {
        let result = bank_lexicon().resolve("zyzzyva", &ResolutionContext::default());
        assert!(matches!(result, ResolutionResult::NotFound));
    }

    #[test]
    fn matching_is_case_insensitive() {
        let result = bank_lexicon().resolve("Bank", &context(&["finance::deposit"]));
        assert!(matches!(result, ResolutionResult::Resolved(_)));
    }

    #[test]
    fn single_binding_resolves_even_without_context() {
        let resolver = ContextResolver::new(LexicalModule {
            language: LanguageId::english(),
            forms: vec![LexicalForm {
                text: "square".to_string(),
                language: LanguageId::english(),
                part_of_speech: Some(PartOfSpeech::Noun),
                bindings: vec![binding("geometry::square", &[])],
            }],
        });
        let ResolutionResult::Resolved(candidate) =
            resolver.resolve("square", &ResolutionContext::default())
        else {
            panic!("expected Resolved");
        };
        assert_eq!(candidate.concept.0, "geometry::square");
    }
}
