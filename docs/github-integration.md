# GitHub integration

## Workflow split

`review-trigger.yml` runs on `pull_request` with no repository/model secrets and no checkout. It only creates a successful workflow run associated with a non-draft PR.

`review.yml` runs on `workflow_run` for that exact trigger workflow. `workflow_run` executes the workflow definition from the default branch, so PR authors cannot rewrite the privileged reviewer. The job checks out `github.sha` (the trusted default-branch revision), builds `rust-review-gate` through sccache, and supplies `OPENROUTER_API_KEY` plus the write-capable `GITHUB_TOKEN` only to the final review step. Third-party setup actions are pinned to immutable commit SHAs.

The privileged workflow never downloads artifacts from the untrusted run and never checks out the PR head. The reviewer gets the PR number from GitHub's workflow metadata, fetches the unified diff plus PR title/body through the Pull Requests API, and sends only those bounded text inputs to OpenRouter. Title/body are explicitly labeled untrusted and are used only as intent/scope context.

If `workflow_run.pull_requests` is empty, the job is skipped rather than guessing a target. This can currently omit some fork-originated runs; adding an authenticated, cross-checked PR resolver is preferable to accepting an untrusted artifact containing a PR number.

## Review posting

Three independent reviewer models run concurrently. Each model currently gets three samples: one general pass followed by two focused passes that reuse the exact large base prompt and add only a tiny review-lens message. Repeat samples can increase one model's coverage but never its vote weight. Findings are normalized onto actual added lines and aggregated only by their primary rule. Two distinct models must agree before the bot posts a consensus finding. Line-local rules require nearby anchors. A relational `helper-complete-migration` finding can match across sibling hunks only when both models name the same exact source `subject` identifier, so unrelated migrations in one file cannot manufacture consensus.

Findings that cannot be safely positioned remain summary-only. `block` and `major` consensus findings make the workflow fail after the review is posted. If no finding reaches consensus, the workflow succeeds without posting a “looks good” comment.

A provider/sample failure is listed in the review summary only when another consensus finding causes a review to be posted. The infrastructure quorum counts distinct models with at least one successful sample, never raw sample count. If fewer models succeed than the vote threshold, the job fails as infrastructure and does not pretend the PR is clean.
