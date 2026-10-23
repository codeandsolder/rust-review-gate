# Historical maintainer replay corpus

These fixtures are exact unified diffs for pre-fix `codeandsolder/sccache-deduplicated` heads that received concrete review feedback on `mozilla/sccache` pull requests. They are intended as blind semantic-review regressions, not prompt examples.

`manifest.json` records the PR, exact head SHA, GitHub review-comment ID, a compact ground-truth summary, and the closest durable project rule. Score a hit only when the reviewer identifies the maintainer's actual failure mechanism; a vaguely related rule or complaint does not count.

Raw model responses and stochastic run outputs are intentionally not committed. Keep durable aggregate results in the project notes, and rerun the current production ensemble against these exact bytes when prompt, aggregation, or model policy changes.

The current cases cover:

- #2910: partial helper migration, displayed-error cause loss, changed failed-probe/write semantics, neighboring-convention drift, and unrelated test churn.
- #2912: doc-comment attachment, incomplete LTO classification, preserving the wrong jobserver boundary, and a regression test with the wrong oracle.
- #2914: redundant mode plumbing, doc-comment attachment, duplicated connect/fallback paths, a behavior-changing hard-coded mode, and unnecessary internal compatibility machinery.
- #2916: validation duplicated above the abstraction that already owns it.

## Strict replay results

Scoring is manual and deliberately strict: a maintainer finding counts only when a consensus finding identifies the same concrete failure mechanism. Sharing a broad rule ID is not enough.

### Baseline: diff only, production 3 x 3 policy

- #2910: 1/5 — caught the failed-check/write semantic regression.
- #2912: 0/4.
- #2914: 1/5 — caught the hard-coded `ReadWrite` handshake regression.
- #2916: 0/1.
- Total: **2/15 (13%)**.

This is substantially worse than the synthetic six-fixture benchmark and is the more important reality check for future reviewer changes.

### Title-context ablation, same 3 x 3 policy

Each replay supplies only the historical PR title as untrusted intent/scope context. Current edited PR descriptions are deliberately excluded so later review fixes cannot leak into the benchmark.

- #2910: 1/5 — retains the failed-check/write hit and suppresses the strongest false claim that persistent capability caching is itself a regression.
- #2912: 0/4.
- #2914: 2/5 — catches the hard-coded `ReadWrite` constructor divergence and the unnecessary optional-state/constructor compatibility machinery in one consensus finding.
- #2916: 0/1.
- Total: **3/15 (20%)**.

PR context is therefore useful input, especially for distinguishing intended behavior from accidental regressions, but it is not a recall solution.

## Single-model brute-force experiment

The single-model full-context experiment using the compact rust-skills2 artifacts at `555620d` is documented in [`bruteforce-555620d.md`](./bruteforce-555620d.md). It compares complete vs routed rule representations, files-first vs rules-first context order, 2-5 discovery samples, reducer behavior, cache/cost, the raw-rule control, and a clean historical control.

## Rejected experiments

- Eight samples/model on #2912 still produced zero consensus. GLM independently found the response-file hole, showing that raw rerolls improve discovery without solving agreement.
- A four-sample policy with a dedicated test-oracle/integration lens produced consensus on the #2912 response-file hole, but also amplified attractive false theories about intentionally cached state in #2910/#2914.
- A discover-then-cross-examine prototype recovered #2910's displayed-error finding, but candidate exposure was too confirmation-biased and promoted multiple speculative findings; it was reverted.

These experiments are intentionally not part of production. Future changes should be judged against these frozen bytes and clean controls rather than tuned until this small corpus happens to pass.
