/-
  eval-ladder - L4 semantic validator root module.

  This module re-exports the obligation and tactic libraries used by the
  curated proof-carrying subset. See `docs/proof_subset_policy.md` at the
  repository root for the scope and selection rubric.
-/

import EvalLadder.Theorems
import EvalLadder.Tactics
import EvalLadder.Fixtures
import EvalLadder.Obligations.ClapRs.Clap5873
import EvalLadder.Obligations.ClapRs.Clap1624
import EvalLadder.Obligations.ClapRs.Clap1710
import EvalLadder.Obligations.ClapRs.Clap1972
import EvalLadder.Obligations.ClapRs.Clap2008
import EvalLadder.Obligations.ClapRs.Clap2075
import EvalLadder.Obligations.ClapRs.Clap2093
import EvalLadder.Obligations.BurntSushi.Ripgrep454
import EvalLadder.Obligations.Fixtures.MilestoneF
