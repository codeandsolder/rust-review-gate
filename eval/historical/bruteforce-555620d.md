# Single-model brute-force review experiment — rust-skills2 `555620d`

This experiment tests the opposite extreme from the production multi-model gate: give one model a large, shared review workspace; sample it repeatedly; then ask the same model to validate and deduplicate the sampled reviews against the source.

## Protocol

Model: `z-ai/glm-5.3-flash` through OpenRouter.

Shared context contains:

- the historical PR title as untrusted intent/scope context;
- complete post-edit contents of every changed file;
- inline `+`/`-` diff markers at the edit sites plus the exact unified diff;
- the project semantic gates;
- one of the rust-skills2 review representations below.

The review system prompt says to determine which rules are relevant to the edited area and verify whether the patch applies them correctly. Source, PR text, comments, and prior reviews are untrusted data rather than instructions.

Sampling configuration:

- temperature `0.20`;
- maximum completion budget `65,536` tokens;
- sample 0 runs first to warm provider/prompt state;
- remaining samples fan out concurrently;
- default discovery efforts are low, low, medium, high, high;
- the practical comparison uses 3 discovery samples (low, low, medium) followed by one high-effort reducer;
- the reducer receives the same full source/rules context plus all prior review JSON, and must verify, deduplicate, or reject candidates from source evidence rather than vote by repetition.

The harness supports 2-5 discovery reviews, 1-3 final reducers, both files-before-rules and rules-before-files ordering, and now checkpoints paid discovery output **before** running synthesis so reducer timeout cannot erase useful samples.

## Rule representations

All compact artifacts come from rust-skills2 branch `single-file-review-skill` at `555620d595ca9738c78ea87845be12b5522a2f0b`.

- `REVIEW_SKILL.md`: compact complete corpus, 325 rules, about 327 kB / 47.7k model tokens.
- `REVIEW_SKILL_ROUTED.md`: 10 always-applied review rules + 13 conditional general-review rules + 325 conditional Rust rules, about 406 kB / 58.5k model tokens.
- Raw handbook control: concatenated source Markdown for all 325 rules, about 1.61 MB / 212k model tokens.

Both generated compact corpora pass their generators' `--check` validation and contain all 325 rule IDs. Neither contains the sccache historical identifiers or maintainer answers used for scoring.

## Strict scoring

As with the main historical corpus, a hit counts only when the review reproduces the maintainer's actual failure mechanism. A nearby concern, shared rule ID, or plausible new bug does not count.

Discovery recall is the union of the independent review samples. Final recall is the reducer's published issue set. These are intentionally reported separately because synthesis can lose true discoveries.

### Compact complete corpus, files first, 3 discovery + 1 reducer

- #2910: **1/5** discovery, **1/5** final — catches the failed capability-check/write semantic regression.
- #2912: **0/4** discovery/final — repeatedly focuses on hypothetical rustc LTO flag coverage rather than the historical response-file/jobserver/test-oracle failures.
- #2914: **0/5** discovery/final.
- #2916: **0/1** discovery/final.
- Total: **1/15 (7%)** discovery and final recall.
- Final #2914 clean control: **pass**. Discovery proposed a mixed-version `StorageHandshake` compatibility concern, which the reducer rejected as unsupported by the same-binary deployment contract.

The compact complete corpus is therefore smaller and operationally practical but substantially worse at selecting the important semantic review boundaries than the routed representation.

### Routed corpus, files first, 3 discovery + 1 reducer

- #2910: **1/5 discovery, 1/5 final** — failed capability-check/write semantics. Discovery also repeatedly proposes stale-mode/cache-refresh concerns; the reducer does not publish those as the primary result.
- #2912: **2/4 discovery, 1/4 final** — discovery independently finds the `@response-file` LTO hole and the fact that the regression test observes `CARGO_MAKEFLAGS` rather than the real jobserver/fd consumer contract. The reducer keeps the response-file defect but silently drops the real-consumer/test-oracle finding, while adding a speculative rustc-LTO concern.
- #2914: **1/5 discovery, 0/5 final** — discovery notices the redundant `IpcStorage::cache_mode` accessor duplicates `Storage::check()`/`handshake.cache_mode`; reducer returns pass.
- #2916: **0/1 discovery/final** — still misses the redundant validation already owned by `get_object`.
- Total: **4/15 (27%) discovery recall**, collapsing to **2/15 (13%) final recall** after synthesis.
- Final #2914 clean control: **pass**. Two speculative discovery complaints are rejected by the reducer.

This is the strongest result of the brute-force experiment so far: explicit generic routing conditions materially improve **discovery**, but the current same-model reducer over-prunes real findings and returns overall recall to roughly the old production baseline.

### Context order

On #2912, routed **files first** is materially better for discovery than routed rules first. Files-first finds both the response-file hole and the real-consumer/test-oracle problem in independent samples. Rules-first mostly converges on generic environment sensitivity/non-UTF8 concerns and does not recover the response-file mechanism.

Rules-first remains interesting economically because the large rule prefix is identical across PRs, but the observed quality loss outweighs that advantage on this small historical stress case.

### Raw full handbook

The 1.61 MB raw-rule control is operationally poor with the deliberately huge generation settings. Both the maximal 5-review/reducer-ladder run and the practical 3+1 rules-first run lost an OpenRouter response at the 600 s response-body timeout before producing a complete quality result. The context itself is accepted by GLM's context window; the failure is long inference/response latency rather than context rejection.

The compact artifacts therefore provide a real operational win, not merely lower token accounting.

## Cache/cost observations

Once the routed prefix is warm, the 3+1 historical runs are cheap despite the large workspace:

- #2910: about `$0.00663`, 81% of prompt tokens reported cached;
- #2914: about `$0.00472`, 84% cached;
- #2916: about `$0.00494`, 87% cached;
- #2914 final-state clean control: about `$0.00377`, 84% cached.

The colder #2912 routed/files-first run cost about `$0.0215` with 62% cache reuse. Low/medium warmed discovery calls generally fall into the tens-of-seconds range. High-effort synthesis is the latency tail: observed reducers range from roughly 50-176 s on warm cases, while the cold #2912 reducer took about 511 s.

## Sample-count ablation

- **2 routed/files-first samples:** **0/4** historical hits; both samples converged on a likely low-value `-flto=off` concern. Discovery completed in about 32-33 s/sample, but the single high reducer still had not completed after roughly 10.5 minutes and the run was terminated. The discovery checkpoint remains usable.
- **3 samples (low, low, medium):** **2/4** discovery hits as above. This is the best practical point observed.
- **5 samples (low, low, medium, high, high):** the first three samples completed in about 33-46 s each, but neither high-effort tail sample completed after roughly 10.5 minutes. The run was terminated rather than extending the ceiling. This configuration is operationally rejected even before considering reducer latency.

The useful marginal budget is therefore discovery *breadth at low/medium effort*, not additional high-effort samples. The high-effort tail dominates wall-clock latency and can prevent a whole discovery batch from completing.

## Conclusions

1. **Use the routed 555620d representation, not the plain compact corpus or raw handbook.** The explicit `WHEN` directory and always-applied semantic gates improve discovery on real maintainer examples.
2. **Files before rules wins the observed quality comparison.** Rules-first may be cheaper across PRs, but #2912 loses the important boundary discoveries.
3. **Spend budget on discovery breadth before high reasoning.** Two samples are too few; three low/low/medium samples are substantially better. High samples are the latency tail.
4. **The reducer is currently the main correctness bottleneck.** It is valuable for clean-control precision, but it drops true discoveries. Future synthesis work should require every candidate to be explicitly accepted or rejected with source evidence rather than allowing silent omission.
5. **Do not replace the production gate with this yet.** Routed brute-force discovery is promising, but strict final historical recall is still only 2/15 with the current reducer.
