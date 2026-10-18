# rust-review-gate

A lean Rust pull-request quality gate combining deterministic checks with narrow, independent LLM reviewers.

Rule inputs:
- **rust-skills2**: an embedded 325-rule snapshot generated from `codeandsolder/rust-skills2` commit `a80bed67fdccbc68320fba946ba819b05f4269ea`. Applicable rule families are selected deterministically from each diff.
- **sccache maintainer-feedback gates**: invariant ownership, failure-path semantic drift, incomplete helper migrations, rendered error context, unnecessary internal compatibility, scope creep, and missing boundary/state regressions.

## Architecture

1. Treat the PR diff as untrusted text; never execute PR-head code in the credential-bearing reviewer.
2. Run deterministic added-line checks as review leads, not automatic defects.
3. Select relevant rust-skills2 rule families from the vendored rulepack.
4. Run independent reviewers concurrently and require structured findings with one **primary rule**, optional supporting rules, concrete evidence, a triggering condition, and a consequence.
5. Reject unknown rule IDs, invalid severities, and findings below 0.70 confidence; snap locations to nearby real added lines.
6. Optionally sample each model multiple times. Sample 1 is a general pass; later samples preserve the full diff/rules as an identical cached message and add only a tiny independent review-lens message. Samples are ORed within that model for coverage but still count as only **one model vote**; they can never manufacture cross-model consensus. A stable OpenRouter `session_id` keeps the repeated prefix on a warm provider.
7. Require distinct models to agree on the same primary rule/file and, by default, the same line neighborhood. Relational helper-migration findings can instead agree across hunks when both models name the same exact source `subject` identifier.
8. Fail closed rather than partially reviewing a diff larger than 220 kB.

The default ensemble is:

- `z-ai/glm-5.3-flash` — mandatory reasoning at low effort
- `meta/muse-spark-1.3-contributor` — mandatory reasoning at low effort
- `~openai/gpt-luna-latest` — reasoning disabled

with a 2-vote consensus threshold. The GitHub workflow currently takes three samples per model at temperature 0.20. Individual sample failures are isolated; the run fails as infrastructure if fewer than two distinct reviewer models produce any successful sample.

## GitHub trust split

`.github/workflows/review-trigger.yml` is a zero-secret `pull_request` signal. `.github/workflows/review.yml` runs only from the trusted default branch via `workflow_run`, checks out trusted code, builds through sccache, and fetches the PR only as a unified diff through GitHub's API. The OpenRouter key and write-capable GitHub token are scoped to the final review step. Setup actions are pinned to immutable commits.

The privileged workflow never checks out, builds, sources, or executes PR-head content. Compile/test/Clippy/Miri for the code under review belong in separate credential-free CI jobs.

## Local use

```bash
export OPENROUTER_API_KEY=...

cargo +1.99.0 run -- review --diff path/to/pr.diff \
  --model 'z-ai/glm-5.3-flash' \
  --model 'meta/muse-spark-1.3-contributor' \
  --model '~openai/gpt-luna-latest' \
  --samples 3 --temperature 0.20

cargo +1.99.0 run -- github \
  --repo owner/repo --pull 123 --min-votes 2 \
  --model 'z-ai/glm-5.3-flash' \
  --model 'meta/muse-spark-1.3-contributor' \
  --model '~openai/gpt-luna-latest' \
  --samples 3 --temperature 0.20 \
  --dry-run
```

Use Rust 1.99 and sccache. Keep `CARGO_TARGET_DIR` outside the checkout.
