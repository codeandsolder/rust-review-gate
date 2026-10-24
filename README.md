# rust-review-gate

A lean Rust pull-request quality gate combining deterministic checks with narrow, independent LLM reviewers.

Rule inputs:
- **rust-skills2**: an embedded 325-rule snapshot generated from `codeandsolder/rust-skills2` commit `a80bed67fdccbc68320fba946ba819b05f4269ea`. Applicable rule families are selected deterministically from each diff.
- **sccache maintainer-feedback gates**: invariant ownership, failure-path semantic drift, incomplete helper migrations, rendered error context, unnecessary internal compatibility, scope creep, and missing boundary/state regressions.

## Architecture

1. Treat the PR diff and author-provided PR title/body as untrusted text; never execute PR-head code in the credential-bearing reviewer. Title/body are used only to understand claimed intent and scope, never as proof or instructions.
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

`.github/workflows/review-trigger.yml` is a zero-secret `pull_request` signal. `.github/workflows/review.yml` runs only from the trusted default branch via `workflow_run`, checks out trusted code, builds through sccache, and fetches the PR unified diff plus title/body through GitHub's API. The OpenRouter key and write-capable GitHub token are scoped to the final review step. Setup actions are pinned to immutable commits.

The privileged workflow never checks out, builds, sources, or executes PR-head content. Compile/test/Clippy/Miri for the code under review belong in separate credential-free CI jobs.

## Local use

```bash
export OPENROUTER_API_KEY=...

cargo +1.99.0 run -- review --diff path/to/pr.diff --context path/to/pr-context.txt \
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

`bruteforce-bench` is an evaluation-only path. `--include-reasoning` asks OpenRouter to return provider-exposed reasoning and persists it in local benchmark JSON for diagnostics; normal runs exclude it. `--final-reducers 0` allows discovery-only trace capture. Raw reasoning traces are intentionally not committed. Output protocols are explicit: `--output-protocol strict` uses native JSON Schema, `tool` forces one schema-bearing tool call, `tool-auto` supplies that tool without `tool_choice`, and `prompt` falls back to prompt-only exact-schema coercion. For prompt fallback, `--schema-position early|late|both` makes output-contract placement explicit; `early` is the default because the free-model ablation recovered materially better semantic recall on Flint, while `late`/`both` remain compatibility probes for stricter formatters. Parsed fallback output is validated against the same exact shape before it counts as a successful sample.

For repository-specific experiments, `--repository-overlay PATH` appends trusted project facts to the rule context without baking them into the universal rule corpus. `eval/historical/sccache-review-overlay.md` is the sccache replay overlay; it covers compiler child-process state, indirect compiler inputs, unhandled-compile fallback, cache-mode handshake ownership, cache-level equivalence, and CI/toolchain contracts. `--max-output-tokens` can lower the completion reservation when screening models with smaller context windows, and `--request-timeout-secs` bounds slow provider calls.

## Historical reality check

`eval/historical/` freezes five revision-correct real pre-fix sccache snapshots and the maintainer findings they later received. The synthetic six-fixture corpus remains useful for fast regression testing, but it is not representative enough to justify strong recall claims.

Under the current 3-sample/model, 2-of-3 production policy, the original four-snapshot replay must be scored against **14 revision-aligned findings**, not 15: one #2912 doc-comment review belonged to an earlier head and is now a separate fixture. The aligned production replay found **2/14 (14%)** from diff-only input and **3/14 (21%)** with historical PR-title context. The separate earlier #2912 doc-attachment snapshot is not retroactively mixed into those completed runs. See `eval/historical/README.md` for provenance and `eval/historical/failure-layer-analysis-2026-10-24.md` for exposed-reasoning diagnosis.


### Experimental single-model brute-force bench

`bruteforce-bench` is an **experimental evaluation path**, not the production GitHub reviewer. It reconstructs full changed files for the frozen historical corpus, inserts inline diff markers, supplies a complete external rule corpus, primes one GLM request, then fans out 2-5 independent reviews before an optional 1-3 reducer stage.

Example using the routed single-file rust-skills2 corpus:

```bash
cargo +1.99.0 run -- bruteforce-bench   --historical eval/historical   --full-rules /path/to/rust-skills2/REVIEW_SKILL_ROUTED.md   --output /tmp/bruteforce.json   --pr 2912 --order files-first   --review-count 3 --final-reducers 1 --skip-prefix-reducers
```

The harness checkpoints completed discovery reviews before synthesis, so a slow reducer cannot erase paid-for discovery output. The `555620d` experiment found that the routed compact corpus improves historical discovery recall, but the current same-model reducer over-prunes true findings; see `eval/historical/bruteforce-555620d.md` for the full comparison and cache/cost data.


Repository-local review behavior belongs in the trusted CI checkout, not the universal rule corpus. The `github` command accepts `--policy PATH` for trusted repo-specific priorities and `--informational-only` for advisory reviews that still post findings but never fail the PR. This is the intended default for fast-moving internal/prototype repositories; external contribution workflows can omit `--informational-only` and use a policy that elevates local style, scope discipline, and maintainer transaction cost. The bundled local workflow also marks the review step `continue-on-error`, so provider/quorum failures stay visible but cannot become a merge gate for this advisory configuration.
