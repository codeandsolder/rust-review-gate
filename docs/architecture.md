# Architecture

## Trust boundary

The reviewer handles pull-request content strictly as data. Its model process receives only a unified diff, deterministic leads, and named review rules. It has no shell/filesystem tools and never executes code from the PR.

GitHub uses a two-workflow split. A zero-secret `pull_request` workflow emits a successful signal. A separate `workflow_run` workflow is loaded from the trusted default branch, checks out only trusted code, builds the reviewer, then fetches the target PR diff through GitHub's API. Repository/model credentials exist only on the review step. Setup actions are pinned to immutable commits.

This avoids both common failure modes: a same-repository PR rewriting a secret-bearing `pull_request` workflow, and a privileged workflow checking out attacker-controlled PR code.

## Rule application

The project embeds a generated 325-rule rust-skills2 snapshot with its source commit. `own-` and `anti-` are baseline families. Error, test, lint, unsafe, async/concurrency, API/types/traits, memory/performance, numeric, serde, pattern, macro, naming, collection, const, docs, observability, and project-structure families are selected from changed text and paths.

The sccache-derived project gates remain a compact layer for recurring review failures that generic guidance does not capture well: invariant ownership, failure-path behavior, actual displayed error information, complete helper migration, internal-only compatibility machinery, state/boundary regressions, and scope discipline.

## Finding contract and consensus

Each finding names exactly one primary rule and may name supporting rules. Unknown rule IDs, malformed severities, empty locations, and confidence below 0.70 are removed before aggregation. Supporting rules never create consensus by themselves.

Model line numbers are treated as hints and snapped to a nearby real added line when possible. Consensus requires distinct models to report the same primary rule in the same file within a small line window. The strongest representative finding supplies the explanation/evidence for the cluster.

A `block` or `major` consensus finding fails the check after the review is posted. Clean PRs receive no noisy approval comment. Provider/model failures are isolated; the run fails as infrastructure only when successful reviewers fall below the configured vote threshold.

## Model profiles

The default reviewers are GLM 5.3 Flash, Muse Spark 1.3 Contributor, and GPT Luna Latest. Luna runs without hidden reasoning because this task is faster and reliable that way. GLM and Muse require reasoning, so they run at low effort with a larger completion envelope. Responses are requested in JSON mode and validated locally.

## Bounded scope

The MVP refuses diffs above 220 kB rather than silently truncating them. Deterministic file/hunk chunking is the next extension for larger PRs. Build/test/Clippy/Miri remain separate credential-free checks; their statuses can later become trusted evidence for the reviewer without widening the model's execution boundary.
