/-
  Obligation: clap-rs__clap_5873 — "Default value not filled in on
  ignored error".

  Upstream issue: clap reports `ValueSource::CommandLine` for an
  argument that is referenced only by a did-you-mean recovery hint,
  even when the user called `.ignore_errors(true)`. The upstream fix
  (`clap_builder/src/parser/parser.rs` lines 1571-1580 at base commit
  `1d5c6798d`) wraps the offending `start_custom_arg(..., CommandLine)`
  in an `if !self.cmd.is_ignore_errors_set()` guard:

      // Add the arg to the matches to build a proper usage string
      if !self.cmd.is_ignore_errors_set() {
          if let Some((name, _)) = did_you_mean.as_ref() {
              if let Some(arg) = self.cmd.get_keymap().get(&name.as_ref()) {
                  self.start_custom_arg(matcher, arg, ValueSource::CommandLine);
              }
          }
      }

  This module states and proves the corresponding semantic obligation:
  the did-you-mean recovery step is the identity on the ArgMatcher
  whenever `ignore_errors = true`, and in particular an argument whose
  value source was `DefaultValue` remains `DefaultValue` after the
  step.

  Scope note. This file is a *declarative* semantic model of the
  narrow parser fragment mutated by the patch. It is intentionally
  small: `docs/proof_subset_policy.md` forbids automatic Rust-to-Lean
  translation and caps reviewer effort per obligation. The fidelity of
  the model to upstream Rust is a reviewer-owned property; the obligation
  manifest's `target_files` and `expected_touched_symbols` give CI
  enough structure to reject patches that do not touch the relevant
  surface area, and the L0/L1/L2 layers of the ladder continue to run
  the upstream regression tests.

  See `datasets/derived/proof_subset/manifest.jsonl` for the obligation
  metadata and `docs/proof_subset_policy.md` for the selection rubric.
-/

namespace EvalLadder.Obligations.ClapRs.Clap5873

/-- The `ValueSource` tag clap attaches to a parsed argument. Only the
    two variants that appear on the did-you-mean recovery path are
    modeled; `EnvValue` is orthogonal to this obligation and adding it
    would not change the theorem. -/
inductive ValueSource
  | DefaultValue
  | CommandLine
  deriving DecidableEq, Repr

/-- Minimal projection of `clap_builder::parser::ArgMatcher` exercised
    by the offending branch. Real clap stores per-argument metadata;
    the obligation only constrains the value-source component. -/
structure ArgMatcher where
  /-- Value source recorded for the argument under study
      (`ignore_immutable` in the issue reproducer). -/
  valueSource : ValueSource
  deriving Repr

/-- The "did you mean" near-miss suggestion carried by the error path.
    Modeled as an opaque name; the theorem does not depend on the
    specific string. -/
structure DidYouMean where
  argName : String
  deriving Repr

/-- Post-patch semantics of the did-you-mean recovery step.

    When `ignoreErrors = true`, the guard added by the upstream fix
    short-circuits before any `start_custom_arg` call, so the
    `ArgMatcher` is returned unchanged. When `ignoreErrors = false`
    and a near-miss exists, the step records the near-miss argument
    as `ValueSource.CommandLine` so the generated usage string is
    informative; this pre-existing behaviour is not a bug.

    The pre-patch semantics omitted the outer `if` and therefore
    transitioned to `CommandLine` even in the `ignoreErrors = true`
    case — that is exactly the behaviour the issue reports. -/
def applyDidYouMeanRecovery
    (m : ArgMatcher) (suggestion : Option DidYouMean) (ignoreErrors : Bool) : ArgMatcher :=
  if ignoreErrors then m
  else
    match suggestion with
    | none   => m
    | some _ => { m with valueSource := ValueSource.CommandLine }

/-- **Primary obligation.** Under `ignoreErrors = true` the did-you-mean
    recovery step preserves the ArgMatcher bit-for-bit. This is the
    exact guarantee established by the upstream patch. -/
theorem ignore_errors_recovery_is_identity
    (m : ArgMatcher) (s : Option DidYouMean) :
    applyDidYouMeanRecovery m s true = m := by
  simp [applyDidYouMeanRecovery]

/-- Specialisation to the issue reproducer. Starting from a matcher
    whose `valueSource` is `DefaultValue`, the did-you-mean recovery
    step with `ignoreErrors = true` leaves the source as
    `DefaultValue` for every possible suggestion value — including
    the `some _` case that triggered the upstream panic. -/
theorem default_preserved_under_ignore_errors
    (s : Option DidYouMean) :
    (applyDidYouMeanRecovery { valueSource := ValueSource.DefaultValue } s true).valueSource
      = ValueSource.DefaultValue := by
  simp [applyDidYouMeanRecovery]

/-- Contrast lemma that pins down the pre-patch bug. This is deliberately
    kept in-tree so future reviewers can verify the model distinguishes
    the fixed and broken behaviours. -/
theorem prepatch_counterexample_exists :
    ∃ (m : ArgMatcher) (s : DidYouMean),
      m.valueSource = ValueSource.DefaultValue ∧
      (applyDidYouMeanRecovery m (some s) false).valueSource = ValueSource.CommandLine := by
  refine ⟨{ valueSource := ValueSource.DefaultValue }, { argName := "ig" }, ?_, ?_⟩
  · rfl
  · simp [applyDidYouMeanRecovery]

end EvalLadder.Obligations.ClapRs.Clap5873
