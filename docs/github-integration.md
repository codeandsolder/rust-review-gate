# GitHub integration

## Workflow split

`review-trigger.yml` runs on `pull_request` with no repository/model secrets and no checkout. It only creates a successful workflow run associated with a non-draft PR.

`review.yml` runs on `workflow_run` for that exact trigger workflow. `workflow_run` executes the workflow definition from the default branch, so PR authors cannot rewrite the privileged reviewer. The job checks out `github.sha` (the trusted default-branch revision), builds `rust-review-gate` through sccache, and supplies `OPENROUTER_API_KEY` plus the write-capable `GITHUB_TOKEN` only to the final review step. Third-party setup actions are pinned to immutable commit SHAs.

The privileged workflow never downloads artifacts from the untrusted run and never checks out the PR head. The reviewer gets the PR number from GitHub's workflow metadata, fetches the unified diff through the Pull Requests API, and sends only that text to OpenRouter.

If `workflow_run.pull_requests` is empty, the job is skipped rather than guessing a target. This can currently omit some fork-originated runs; adding an authenticated, cross-checked PR resolver is preferable to accepting an untrusted artifact containing a PR number.

## Review posting

Three independent reviewers run concurrently. Findings are normalized onto actual added lines and aggregated only by their primary rule. Two distinct models must agree on primary rule, file, and nearby line before the bot posts an inline consensus finding.

Findings that cannot be safely positioned remain summary-only. `block` and `major` consensus findings make the workflow fail after the review is posted. If no finding reaches consensus, the workflow succeeds without posting a “looks good” comment.

A provider failure is listed in the review summary only when another consensus finding causes a review to be posted. If fewer reviewers succeed than the vote threshold, the job fails as infrastructure and does not pretend the PR is clean.
