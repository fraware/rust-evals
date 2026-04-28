# Public terminology guide

This document standardizes wording used in public-facing documentation and release
artifacts.

## Canonical mappings

- `flagship` -> `primary evaluation cohort`
- `sealed` -> `frozen reproducibility snapshot`
- `strict gate` -> `publication-threshold gate`
- `release profile` -> `repository-release profile`
- `gold patch` -> `reference patch`
- `results_astropy` -> `IssueRelevanceValidationArm`
- `results_regression_fail` -> `NegativeControlRegressionArm`

## Naming policy for agent identifiers

Use meaningful neutral naming in narrative prose. Prefer descriptors such as:

- `upstream-resolved candidate source`
- `comparative candidate source`
- `reference agent panel source`

Keep low-level IDs (for example `gru`, `honeycomb`, `sweagent`) only in tables,
machine-readable artifacts, or exact command examples where traceability requires
them.

## Style rules

- Prefer descriptive labels over pipeline shorthand.
- Explain control-arm behavior explicitly when a mechanism is intentionally
  stress-oriented.
- Keep reason-code identifiers in dedicated code blocks or appendix sections;
  use plain language in summary sections first.
