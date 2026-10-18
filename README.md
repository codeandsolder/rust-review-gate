# rust-review-gate

A deliberately small Rust pull-request reviewer combining deterministic source checks with narrow LLM review lenses.

Rule inputs:
- **rust-skills2**: correctness, error handling, async, unsafe, API, testing, performance, and anti-pattern guidance.
- **sccache maintainer-feedback gates**: invariant ownership, failure-path semantic drift, incomplete helper migrations, rendered error context, unnecessary internal compatibility, scope creep, and missing boundary/state regressions.

## Architecture

1. Parse only the PR diff. The model-review job does not execute PR code.
2. Run deterministic added-line checks as review leads, not automatic defects.
3. Ask a model for concrete findings using stable named review-rule IDs.
4. Require structured JSON with file/line/evidence/trigger/consequence.
5. Benchmark models against a curated corpus based on our actual historical review failures.
6. Keep compile/test/Clippy jobs credential-free and separate from the OpenRouter review job.

This is intentionally closer to a deterministic quality gate with contextual judgment than a general autonomous coding agent.

## Local use

```bash
export OPENROUTER_API_KEY=...
cargo +1.99.0 run -- review --diff path/to/pr.diff --model deepseek/deepseek-v4.1-flash
cargo +1.99.0 run -- bench --fixtures eval/fixtures \
  --model deepseek/deepseek-v4.1-flash \
  --model '~openai/gpt-luna-latest'
```

`review` and `bench` never execute code from the reviewed diff.
