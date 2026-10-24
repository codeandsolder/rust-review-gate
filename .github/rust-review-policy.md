# Local rust-review-gate policy

This repository's AI review is advisory. Prioritize defects that could make the reviewer miss real regressions, report false findings, violate the trusted/untrusted boundary, corrupt consensus, silently lose candidates, mis-handle provider failures, or make evaluation results invalid.

- Treat style, formatting, naming taste, and cosmetic refactors as non-findings unless they directly create ambiguity or break a repository-enforced contract.
- Do not preserve internal compatibility merely because an old internal shape existed. Prefer the simplest complete internal design unless there is evidence of an actual external consumer.
- Report unnecessary scope churn only when it materially increases review risk or obscures the behavioral change; trivial disposable cleanup is not worth a finding here.
- Prefer deterministic/schema-enforced correctness over prompt-only conventions when the output or control flow can be structurally constrained.
- For evaluation code, benchmark provenance, fixture isolation, scoring semantics, and accidental answer leakage are correctness properties, not bookkeeping nits.
