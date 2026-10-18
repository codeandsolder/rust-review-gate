# GitHub integration

The secret-bearing review job never checks out or executes the pull-request head. It checks out the trusted base SHA, builds `rust-review-gate` through sccache, fetches the PR unified diff through GitHub's API, and sends only text to OpenRouter.

The first production policy uses three independent reviewers and requires two votes on the same named rule in the same file/nearby line window. Model line numbers are deterministically snapped to nearby real added lines before consensus and before GitHub positioning. Findings that cannot be safely positioned remain summary-only rather than becoming invalid inline comments.

A consensus `block` or `major` finding makes the workflow fail after the review has been posted. Clean PRs do not receive a noisy “looks good” comment.

Fork PRs are deliberately skipped by the simple `pull_request` workflow because repository secrets are unavailable there. Supporting untrusted forks safely should use a two-workflow artifact handoff (`pull_request` produces text-only review input; a trusted `workflow_run` consumes that artifact) rather than switching to `pull_request_target` and risking execution of attacker-controlled code under secrets.
