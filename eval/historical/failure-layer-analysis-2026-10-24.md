# Failure-layer analysis from exposed GLM reasoning — 2026-10-24

This note diagnoses the single-model brute-force reviewer by layer rather than treating every historical miss as a model-intelligence failure. Original benchmark runs excluded provider reasoning; diagnostic reruns set OpenRouter reasoning output on and persist `reasoning` / `reasoning_details` only in local cache. Raw traces remain uncommitted.

## First: one evaluation label was wrong

GitHub review comment `4308066522` (the #2912 doc-comment attachment complaint) records `original_commit_id = 2a92befa44a88386c991b8bc589b2a8418d1496b`. The old corpus attached it to later head `73604403f3185e43aeb906c86eb91ca89e0ee67e`, where `needs_lto_jobserver` has already been moved above the `handle_compile_result` doc comment. That apparent miss was a correct abstention against a mismatched fixture.

The corpus now stores `2a92bef…` as its own snapshot. Legacy four-snapshot scores are therefore reported against the 14 findings actually aligned with those four heads; the fifth snapshot is not retroactively mixed into old runs. A corrected rules-first diagnostic replay of `2a92bef…` then missed the doc attachment in all three discovery samples, so the original apparent miss was an eval bug, but the underlying reviewer weakness is still real on the correct revision.

## Layer map

| Case | Maintainer mechanism | What the trace shows | Primary failing layer |
| --- | --- | --- | --- |
| #2910 | incomplete `record_write_result` migration | Rules-first samples inspect `record_write_result` and `write_remaining_levels_async`; one audit explicitly calls the helper a single owner replacing duplicated logic instead of noticing sibling manual stats bookkeeping | rule grounding / wrong abstraction; maintainability objective underweighted |
| #2910 | displayed error loses cause | Files-first missed it; rules-first independently reconstructs the exact `anyhow::Context` / `Display` failure and even finds the pre-existing assertion that breaks | effective-attention / prompt position, not missing rule knowledge |
| #2910 | neighboring `as u64` convention | Exposed reasoning explicitly notices `u64::try_from(...).unwrap_or(u64::MAX)` and calls it an improvement | deliberate judgment against `analogous-semantics`; objective mismatch with maintainer-style consistency review |
| #2910 | failed probe now prevents write | Found reliably | working layer |
| #2910 | unrelated test refactor | Audit considers test refactors and calls them behavior-neutral / scope OK | rule interpretation + top-level objective mismatch: behavior-neutral churn is exactly what `scope-discipline` is meant to catch |
| #2912 `2a92bef…` | doc comment for `handle_compile_result` is stolen by inserted helper | Corrected rules-first exposed trace: 0/3 samples report it; reasoning explicitly says `review-doc-attachment` is not applicable / no structural doc move even though the bad attachment is visible | genuine local rule-grounding + stochastic gate-activation failure |
| #2912 `7360440…` | visible-argv LTO classification misses implicit forms | One routed run finds `@rsp` independently in multiple samples; exposed rerun instead fixates on speculative rustc `-C lto` and never activates the response-file route | stochastic rule activation; coverage is prose, not enforced routing |
| #2912 `7360440…` | wrong jobserver boundary (`CARGO_MAKEFLAGS` vs real consumer state) | Reasoning discusses `MAKEFLAGS`, fds and `share_jobserver`, but repeatedly says implementation is unknown / cannot verify | missing unchanged-context closure around `share_jobserver`, configure/spawn and consumer contract |
| #2912 `7360440…` | regression test checks wrong oracle | Earlier medium sample finds this directly; reducer neither keeps it nor lists it as discarded | reducer schema allows silent candidate loss |
| #2914 | redundant `cache_mode` accessor/path | Files-first discovery notices duplication; reducer rejects it because other consumers may exist outside visible evidence | context closure + maintainability objective / evidence threshold |
| #2914 | doc comment retargeted to `StatsOutput` | Old files-first audit explicitly says no misplaced docs; rules-first exposed trace spots the exact attachment defect | prompt position plus stochastic local reasoning |
| #2914 | duplicated Text/JSON connect/fallback flow | Trace activates duplicate-logic review but focuses on the extracted `request_stats_on` helper and declares duplication solved; it does not compare the outer `ShowStats` branches | rule grounding at wrong abstraction boundary |
| #2914 | `with_listener` hardcodes `ReadWrite` | Files-first largely misses third-file `server.rs`; rules-first samples and reducer recover the exact semantic regression | effective-attention / prompt position |
| #2914 | constructors + `Option` preserve internal compatibility | Reasoning activates `review-internal-compatibility`, notices wrappers, then softens because unseen callers may exist | missing repo-wide call-site closure plus objective/evidence threshold |
| #2916 | new test block duplicates validation already owned by `get_object` | Rules-first trace sees `get_object`, notes its `Stored`/mode behavior, and explicitly activates `review-invariant-owner`; it nevertheless maps the invariant to “ZIP format correctness,” labels duplicate logic absent and scope N/A, then praises the round-trip test | genuine relational reasoning / rule-grounding failure |

## What files-first was actually breaking

The prompt assembler does include every changed file and appends the exact unified diff. There is no explicit source truncation. However, files-first layout places all edited files + diff before the ~406 kB routed corpus. On multi-file PRs the model behaves as though later files/regions disappeared:

- #2910: first `multilevel.rs` dominates; separate test-file scope churn is missed.
- #2914: `ipc_storage.rs` / `commands.rs` dominate; third-file `server.rs` semantic regression is missed.
- #2916: model claims only `Cargo.lock` / no relevant source was supplied even though `Cargo.toml` and `cache_io.rs` are present.

Moving the same bytes to rules-first, so edited files and the exact diff are at the tail, changes behavior immediately:

- #2910 recovers the displayed-error defect.
- #2914 recovers the hard-coded `ReadWrite` regression and, in an exposed trace sample, the doc attachment defect.
- #2916 now describes `cache_io.rs`, `get_object`, tests, Cargo.toml and Cargo.lock correctly.

The prompts are far below GLM 5.3 Flash's advertised hard context limit, so this is best described as effective-attention / lost-in-the-middle behavior rather than literal context truncation.

## Rules are present; routing is not enforced

The routed corpus and project gates already contain the relevant policies: displayed error, doc attachment, internal compatibility, invariant ownership, scope discipline, real-consumer boundary, implicit inputs, helper-complete migration, analogous semantics, and others. Missing rules are not the main problem.

The problem is that `coverage` is free-form prose. A sample may claim to have performed a “rule sweep” without producing a machine-checkable disposition for the conditional gates. #2912 demonstrates the resulting stochasticity: one run finds response files and the real-consumer test oracle; another run with the same context spends its attention on rustc `-C lto` and never forms those candidates.

A stronger pipeline should produce an explicit routing artifact for the small general-review gate set, e.g. `{rule_id, applicable: yes|no|uncertain, evidence}`. The reviewer can still reason freely about the 325 Rust rules, but the semantic gates that define maintainer-quality review should not be optional prose. Some gates should also become deterministic where feasible: doc-comment attachment can be checked from pre/post syntax association, avoiding stochastic LLM routing entirely.

## The top-level objective conflicts with part of the benchmark

The brute-force system prompt says to report only concrete patch-caused defects with a realistic failure mechanism and to prefer semantic defects over style or generic best practices. Several maintainer comments are deliberately broader code-review quality findings: scope churn, duplicated control flow, redundant validation, internal compatibility machinery, and neighboring convention drift.

That makes some current “misses” policy-compliant. Named review gates should explicitly override the generic runtime-defect filter: a concrete maintainability/scope/documentation violation is a valid finding even when it does not crash at runtime. Category/severity can distinguish it from behavioral correctness.

## The reducer has a structural correctness bug

In the #2912 routed files-first run, discovery sample 2 produced the real test-oracle finding. The reducer did not publish it and did not include it in `discarded_candidates`. The prompt says to verify every candidate, but nothing validates that instruction.

Reducer input should first be deterministically normalized into candidates with stable IDs. Output must account for every ID exactly once: `accepted`, `rejected` with source evidence, or `merged_into:<id>`. Missing IDs should make synthesis invalid rather than silently deleting discoveries.

## Context closure is still required even with full changed files

Changed files are not always enough to verify an invariant:

- #2910 displayed-error semantics depend on an unchanged log/render consumer.
- #2912 jobserver correctness depends on unchanged `share_jobserver`, environment construction/spawn behavior and the external compiler/linker consumer.
- #2914 internal-wrapper necessity depends on actual repo-wide callers.

A deterministic context-expansion pass should follow changed identifiers to unchanged callers/implementations (initially `rg` is sufficient): include consumers of new helpers/types, implementations of changed wrapper methods, error render/log sites, and call sites relevant to internal compatibility.

## Recommended next pipeline

1. **Rules/gates before source; source + exact diff at the absolute tail.** Put generated/lock files after hand-written source or summarize them unless the diff touches semantics there. A small always-on gate directory may be repeated at the tail.
2. **Three low/low/medium discovery samples.** High-effort samples remain a latency pathology without demonstrated recall benefit.
3. **Explicit semantic-gate routing/dispositions.** Do not trust free-text coverage claims.
4. **Deterministic unchanged-context closure** for changed symbols and real consumers.
5. **Allow named maintainability/scope/doc gates to produce findings** even without a runtime failure mechanism.
6. **Deterministic candidate union with stable IDs.** Reducer must disposition every ID; silent omission is invalid.
7. Only then retune sampling/model/reducer reasoning. The trace evidence says adding more inference before fixing these structural layers would spend money around avoidable pipeline failures.
