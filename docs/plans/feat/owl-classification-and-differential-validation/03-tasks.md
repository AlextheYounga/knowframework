# Tasks

## Implementation

1. [ ] Define shared finite corpus entries, canonical ordering, normalized
   result categories, and explicit skip/mismatch records in `know-owl`.
2. [ ] Extend the rustdl adapter to evaluate all ordered named-concept pairs
   through the semantic bridge, quotient complete mutual entailments into
   equivalence sets, and apply deterministic transitive reduction for direct
   parent/child edges; require complete consistency and pairwise results before
   returning sorted `ClassificationResult` values, otherwise propagate
   Unsupported/ResourceLimit/InternalError without partial vectors.
3. [ ] Generate corpus queries from named vocabulary and existing axiom
   expressions, including relation-bearing entries without synthesizing new
   expression structure.
4. [ ] Evaluate BooleanReasoner and RustdlReasoner against the corpus, treating
   only explicit non-overlap Unsupported outcomes as skips; make
   ResourceLimit/InternalError, generated IllTyped queries, and complete
   category differences deterministic nonzero failures.
5. [ ] Replace the `diff-reasoner` CLI stub with the comparison workflow and
   name the sole external-validation flag `--external`; without it do not
   inspect `KNOW_ROBOT`.
6. [ ] Add the opt-in ROBOT/HermiT oracle adapter discovered via `KNOW_ROBOT`,
   using inconsistency-first positive/negative dual consistency probes and
   normalized categories; report missing/unusable configuration as a clear
   nonzero CLI error and as an explicit skip in opt-in automated tests.
7. [ ] Update the existing `Classify` command to use the supported rustdl
   classification path where its backend completeness contract permits it.
8. [ ] Add focused reasoner, corpus, CLI, and optional-oracle tests covering
   directness, equivalence, skips, normalized categories, deterministic output,
   mismatch exit behavior, and unavailable-oracle diagnostics.

## Validation

1. [ ] Verify role-free classification on geometry returns the expected direct
   parents/children/equivalences and excludes indirect hierarchy members.
2. [ ] Verify relational modules and relational queries are reported as
   unsupported/skipped rather than as Boolean/Rustdl mismatches.
3. [ ] Verify complete `Unknown` remains distinct from Unsupported,
   ResourceLimit, and InternalError; only non-overlap Unsupported is skipped,
   while operational failures and generated IllTyped queries fail.
4. [ ] Verify repeated `diff-reasoner` runs over the same module produce
   identical ordered output and exit success when no applicable mismatch exists,
   and verify no `KNOW_ROBOT` discovery occurs without `--external`.
5. [ ] Verify a deliberately divergent complete category produces a nonzero
   CLI exit and identifies the query and both normalized categories.
6. [ ] Verify `--external` discovery, HermiT invocation, inconsistency-first
   positive/negative dual probes, normalized external results, and clear
   nonzero behavior for absent/unusable `KNOW_ROBOT`; verify opt-in tests skip
   clearly when the environment is absent.
7. [ ] Run the focused and workspace validation commands required by the
   repository after implementation; do not treat external-oracle absence as a
   normal-build failure.

## Completion

1. [ ] Update `README.md` if the new command or opt-in oracle requires user
   documentation; do not add external tooling as a normal setup requirement.
2. [ ] Remove obsolete generated/untracked Acta planning files under
   `docs/agents/plans/feat/owl-classification-and-differential-validation/`.
3. [ ] Review the final diff for deterministic ordering, explicit diagnostics,
   no partial Complete classifications, and no implementation outside scope.

## Completion notes

No completion notes yet; this record is awaiting implementation and human
approval.
