# Free-model output protocol experiment — 2026-10-24

## Question

Can the routed single-file reviewer use current OpenRouter `:free` models that do not advertise native structured outputs, and where should a prompt-only schema live?

This is an evaluation-only experiment. Production review behavior is unchanged.

## Protocols

The benchmark now supports four output transports, all feeding the same strict local result validator:

- `strict`: native `response_format: json_schema` with strict schema and parameter-aware routing.
- `tool`: one schema-bearing `submit_review` / `submit_reduction` function, with a named forced tool choice.
- `tool-auto`: the same schema-bearing function without a `tool_choice` parameter; the prompt requires exactly one call.
- `prompt`: no structured-output or tool API requirement. The model is told it must emit exactly the supplied JSON Schema. The parser still rejects missing/extra keys, wrong types, malformed enums, prose, or otherwise nonconforming output.

Prompt-only placement is separately controllable:

- `early`: full output contract before the review objective, terse reminder at the response boundary.
- `late`: full output contract only at the response boundary.
- `both`: full contract both before the objective and at the response boundary.

Requests with reasoning disabled now omit reasoning-control parameters entirely. The review workspace and per-sample instruction are combined into one user turn so strict alternation-only chat backends are not excluded by harness formatting.

## Current free-model compatibility screen

The screen enumerated the 17 `:free` model IDs exposed by OpenRouter during the run. The corrected early #2912 snapshot (`2a92bef…`) was used as the cheap screen because its aligned maintainer finding is local and unambiguous: a new helper steals `handle_compile_result`'s rustdoc.

| Model | Prompt early | Prompt late | Forced tool | Tool auto |
|---|---|---|---|---|
| cohere/north-mini-code:free | timeout | timeout | valid | valid |
| dots-studio/dots-3-note-preview:free | timeout | timeout | routing-incompatible | timeout |
| google/gemma-4-26b-a4b-it:free | upstream rate-limit | upstream rate-limit | upstream rate-limit | upstream rate-limit |
| google/gemma-4-31b-it:free | upstream rate-limit | upstream rate-limit | upstream rate-limit | upstream rate-limit |
| liquid/lfm-2.5-2.6b:free | context limit | context limit | context limit | context limit |
| nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free | timeout | no content / upstream exhaustion | routing-incompatible | timeout |
| nvidia/nemotron-3-super-120b-a12b:free | provider 404 | provider 404 | provider 404 | provider 404 |
| nvidia/nemotron-3-ultra-550b-a55b:free | timeout | valid | timeout | valid |
| nvidia/nemotron-3.5-content-safety:free | context limit at 8k reserve | context limit at 8k reserve | n/a | n/a |
| nvidia/nemotron-3.5-lightning:free | timeout | timeout | valid | timeout |
| nvidia/nemotron-3.5-super-vl:free | timeout | timeout | timeout | timeout |
| poolside/laguna-s-2.1:free | timeout | timeout | routing-incompatible | timeout |
| poolside/laguna-xs-2.1:free | valid | valid | routing-incompatible | schema-invalid |
| recursive/flint-v1.0:free | output budget exhausted | output budget exhausted | n/a | output budget exhausted |
| thinkingmachines/inkling-small:free | agentic-harness-only | agentic-harness-only | n/a | agentic-harness-only |
| thinkingmachines/inkling:free | agentic-harness-only | agentic-harness-only | n/a | agentic-harness-only |
| xingchen-agi/xing4.0-29b-a4b:free | upstream rate-limit | timeout | routing-incompatible | timeout |

Notes:

- `tools` / `tool_choice` in model metadata does not guarantee that the free endpoint accepts a named forced `tool_choice`. Several endpoints fail OpenRouter's Tool Compatibility routing step for the named function object.
- North Mini Code and Nemotron 3.5 Lightning successfully returned strict locally validated function arguments under forced-tool mode even though neither advertises native structured outputs.
- Nemotron 3 Ultra succeeded with prompt-late and tool-auto.
- Liquid's 65,536-token context cannot fit the routed corpus plus edited source. This is a real architecture mismatch, not an output-protocol failure.
- The content-safety model was retried with only a 4,096-token completion reserve after fixing role alternation. It then produced its native `User Safety: safe / Response Safety: safe` classification rather than review JSON, so it is semantically the wrong model for this task.
- Gemma 4 26B/31B were retried after the first screen and remained unavailable because the shared Google free endpoint returned 429s. They are unscored, not semantic zeroes.

## Schema-position result

The placement experiment shows that formatting reliability and review attention are different objectives.

For Flint with a 32,768-token completion budget and two independent samples on early #2912:

- **early: 2/2 exact doc-attachment hits**, 325 s total.
- **late: 0/2 doc-attachment hits**, 196 s total; both samples instead pursued LTO-indirection/fallback issues.

For Laguna XS:

- early two-sample run: 0/2 usable (one timeout, one missing required `validation_state`).
- late two-sample run: 2/2 schema-valid, but neither found the doc-attachment mechanism.
- both, one sample: schema-valid in 45 s, but still missed the doc bug.

For Flint, `both` produced a valid answer in 82 s but again pursued LTO semantics rather than the doc attachment. Repeating the full schema at the tail therefore changed the semantic search trajectory; the extra schema was not neutral.

For Nemotron 3 Ultra, early and late each produced only one usable response in the two-sample confirmation because the other request timed out. The valid late response arrived faster. `both` timed out in its one-sample probe.

**Decision:** prompt-only fallback defaults to `early`: full schema before the review objective plus only a terse reminder at the output boundary. `late` and `both` remain explicit compatibility options for endpoints whose formatting is unreliable with early placement. Do not infer semantic model quality from protocol success alone.

## Important failure distinction

A failed benchmark call is classified by layer rather than collapsed into zero recall:

1. context-window incompatibility;
2. endpoint/provider/rate-limit failure;
3. output transport/format failure;
4. completed review that misses the maintainer mechanism;
5. completed review that finds the same concrete mechanism.

Flint is the clearest example of why this matters: at 8,192 completion tokens it spent the entire output budget reasoning, explicitly noticed the stolen-doc-comment problem in the exposed provider trace, and never emitted a final result. At 32,768 tokens with early placement it emitted the exact finding in both samples.

## Quality replay

A one-sample-per-snapshot replay is used only to estimate whether protocol-compatible free models have useful semantic signal without immediately exhausting shared free-provider rate limits. Multi-sample quorum remains the standard for production/reliable-model evaluation.

Results are filled from revision-correct snapshots only and scored by exact concrete mechanism, never by merely sharing a rule label.

### Current semantic signal from the free field

Protocol success is only an admission ticket. On the revision-correct historical cases, the useful signal so far is sparse:

- **Recursive Flint 1.0 + prompt/early + 32,768 output tokens** is the cleanest positive result: both independent early-#2912 samples identify the exact `doc-attachment` mechanism. The same model with the full schema only at the tail, or repeated at both ends, instead pursues LTO/fallback issues and misses the doc bug.
- **Nemotron 3 Ultra + prompt** has useful semantic signal but is slow/flaky. It finds the #2910 failed-probe/write semantic regression in the earlier multi-sample run, and on #2912 it independently identifies response-file/encoded LTO inputs and the test's wrong jobserver oracle. It still misses the early doc-attachment bug and frequently hits the request timeout.
- **Laguna XS + prompt/late** can produce valid answers but is noisy. It found the wrong-oracle family on later #2912 in one multi-sample run, while a #2914 single sample confidently invented two missing-comma compile failures rather than finding the maintainer mechanisms. Shared-pool 429s make sustained replay unreliable.
- **North Mini Code** and **Nemotron 3.5 Lightning** prove that schema-bearing tool calls can rescue models without native structured outputs. On the historical replay their usable samples were mostly empty findings; harder samples frequently timed out or exhausted the completion budget before the tool call.
- **Xing4** completed four strict-schema snapshots in the earlier strict sweep but did not identify an aligned maintainer mechanism on those completed cases. Its output contains plausible review-shaped findings, which makes it a useful warning against equating structured output with review quality.

No free model/configuration in this experiment is currently a production replacement for the paid ensemble. Flint is worth retaining as an opportunistic specialist/extra sampler when free inference is available; Nemotron 3 Ultra is worth future reruns with tighter context and a longer latency budget. Free-provider failures are kept distinct from semantic misses.

## Paid/control comparison on the same routed corpus

A fresh single-sample replay with the sccache overlay and corrected request profiles gives a useful low-cost comparison. These numbers are discovery-only and should not replace the stronger multi-sample result where available.

- **Muse Spark 1.3 Contributor**: one-sample replay finds the early #2912 doc attachment, later #2912 response-file LTO hole, and #2914 hard-coded `ReadWrite` regression: **3/15 aligned**. More importantly, the completed three-sample Muse replay on the same routed setup reaches **7/15 aligned mechanisms**: #2910 helper-migration, displayed-error and failed-probe/fallback; early #2912 doc attachment; later #2912 response-file classification and wrong test oracle; #2914 hard-coded `ReadWrite`.
- **GPT Luna Latest**: fresh one-sample replay finds #2910 failed-probe/write semantics and displayed-error loss, later #2912 response-file LTO, and #2914 hard-coded `ReadWrite`: **4/15 aligned**. It remains fast and conservative; #2916 is still missed.
- **DeepSeek V4.1 Flash** under its corrected profile (`disable_reasoning`, native strict schema) timed out at the 240 s request ceiling on both #2910 and the tiny early-#2912 snapshot before producing a result. The remaining snapshots were not burned after the repeated failure. DeepSeek is therefore **unscored / operationally unsuitable in this configuration**, not `0/15`.

The larger conclusion is that the routed corpus + repository overlay materially improves useful coverage, but model choice still matters. Muse currently has the strongest observed historical recall; Luna contributes fast, genuinely independent hits. Free models are best treated as opportunistic extra discovery rather than quorum members until their endpoint reliability and recall improve.
