# Architecture

## Security boundary

The model-review process receives text only: unified diff, deterministic leads, and named review rules. It does not receive shell tools and never executes PR code.

Compilation, tests, Clippy, Miri, or project-specific validation belong in separate CI jobs without `OPENROUTER_API_KEY`. Their statuses can later be fed into the reviewer as evidence.

This is deliberate: PR content is attacker-controlled, and a tool-using reviewer with repository secrets would otherwise create a prompt-injection-to-credential-exfiltration path.

## Review lenses

The stable machine-facing rule IDs are distilled from the current rust-skills2 workflow and recurring sccache maintainer feedback. Deterministic scans are review leads only; a new `unsafe` block or `unwrap` is not automatically a defect without context.

The reviewer must identify a triggering condition, consequence, and specific evidence, and abstain below the confidence threshold.

## Planned production gate

Two stages:

1. broad cheap candidate generation by one or more fast models;
2. validation/deduplication of candidate findings, requiring either model agreement or a verifier decision before a blocking conclusion.

The eval corpus selects models and thresholds empirically rather than by reputation.
