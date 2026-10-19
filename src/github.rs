use std::{env, fmt::Write as _, time::Duration};

use anyhow::{Context, Result, anyhow};
use reqwest::{Client, StatusCode, header};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{diff::ParsedDiff, review};

const MAX_INLINE_COMMENTS: usize = 50;

pub async fn run(
    repo: &str,
    pull_number: u64,
    models: &[String],
    min_votes: usize,
    temperature: f32,
    dry_run: bool,
    samples: usize,
) -> Result<()> {
    validate_repo(repo)?;
    if min_votes == 0 || min_votes > models.len() {
        return Err(anyhow!(
            "min-votes must be between 1 and the model count ({})",
            models.len()
        ));
    }

    let github = GithubClient::from_env()?;
    let metadata = github.pull_metadata(repo, pull_number).await?;
    let head_sha = metadata.head_sha.clone();
    let diff = github.pull_diff(repo, pull_number).await?;
    let context = metadata.review_context();
    let parsed = ParsedDiff::parse(&diff);
    let output = review::review_text_with_context(
        &diff,
        Some(&context),
        models,
        temperature,
        min_votes,
        samples,
    )
    .await?;
    let successful_models = review::successful_model_count(&output.runs);
    if successful_models < min_votes {
        let failed = output
            .failures
            .iter()
            .map(failure_label)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(anyhow!(
            "only {successful_models} distinct reviewer model(s) succeeded, below min-votes {min_votes}; failed samples: {failed}"
        ));
    }
    let current_head_sha = github.pull_head_sha(repo, pull_number).await?;
    if current_head_sha != head_sha {
        return Err(anyhow!(
            "pull-request head changed during review ({head_sha} -> {current_head_sha}); discarding stale findings"
        ));
    }

    let (payload, blocking) = build_review_payload(&output, &parsed, &head_sha)?;

    if dry_run {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if !output.consensus.is_empty() {
        github
            .post_review(repo, pull_number, &payload)
            .await
            .context("post GitHub pull-request review")?;
    } else {
        eprintln!("no consensus findings; skipping empty PR comment");
    }

    if blocking > 0 {
        return Err(anyhow!(
            "{blocking} blocking/major consensus finding(s) remain"
        ));
    }
    Ok(())
}

fn build_review_payload(
    output: &review::ReviewOutput,
    parsed: &ParsedDiff,
    head_sha: &str,
) -> Result<(Value, usize)> {
    let mut inline = Vec::new();
    let mut summary_only = Vec::new();
    for finding in &output.consensus {
        let is_added = parsed
            .added_lines
            .iter()
            .any(|line| line.file == finding.file && line.line == finding.line);
        if is_added && inline.len() < MAX_INLINE_COMMENTS {
            inline.push(json!({
                "path": finding.file,
                "line": finding.line,
                "side": "RIGHT",
                "body": format_finding(finding),
            }));
        } else {
            summary_only.push(format!(
                "- `{}` `{}`:{} — **{}**: {}",
                finding.rule, finding.file, finding.line, finding.title, finding.explanation
            ));
        }
    }

    let blocking = output
        .consensus
        .iter()
        .filter(|finding| matches!(finding.severity.as_str(), "block" | "major"))
        .count();
    let successful_models = review::successful_model_count(&output.runs);
    let mut body = format!(
        "rust-review-gate: {} consensus finding(s) from {} successful model(s), {} successful sample(s); {} blocking/major.",
        output.consensus.len(),
        successful_models,
        output.runs.len(),
        blocking
    );
    if !output.failures.is_empty() {
        body.push_str("\n\nReviewer failures (excluded from consensus):\n");
        for failure in &output.failures {
            writeln!(
                &mut body,
                "- `{}` unavailable; excluded from consensus",
                failure_label(failure)
            )?;
        }
    }
    if !summary_only.is_empty() {
        body.push_str("\nFindings that could not be safely anchored to an added line:\n");
        body.push_str(&summary_only.join("\n"));
    }
    if output.consensus.is_empty() {
        body.push_str(" No finding reached the configured consensus threshold.");
    }

    let payload = json!({
        "commit_id": head_sha,
        "event": "COMMENT",
        "body": body,
        "comments": inline,
    });
    Ok((payload, blocking))
}

fn failure_label(failure: &review::ModelFailure) -> String {
    failure.sample.map_or_else(
        || format!("{}#unknown-sample", failure.model),
        |sample| format!("{}#{}", failure.model, sample + 1),
    )
}

fn format_finding(finding: &crate::aggregate::ConsensusFinding) -> String {
    let mut body = format!(
        "**{}** (`{}`, {} model votes / {} sample hits)\n\n{}\n\nEvidence: {}",
        finding.title,
        finding.rule,
        finding.votes,
        finding.sample_votes,
        finding.explanation,
        finding.evidence
    );
    if let Some(test) = &finding.suggested_test {
        body.push_str("\n\nSuggested regression test: ");
        body.push_str(test);
    }
    body
}

fn validate_repo(repo: &str) -> Result<()> {
    let mut parts = repo.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return Err(anyhow!("repo must be in owner/name form"));
    }
    if !owner
        .chars()
        .chain(name.chars())
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(anyhow!("repo contains unsupported characters"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct PullResponse {
    head: PullHead,
    title: String,
    body: Option<String>,
}

#[derive(Deserialize)]
struct PullHead {
    sha: String,
}

struct PullMetadata {
    head_sha: String,
    title: String,
    body: String,
}

impl PullMetadata {
    fn review_context(&self) -> String {
        format!("PR title: {}\n\nPR description:\n{}", self.title, self.body)
    }
}

struct GithubClient {
    http: Client,
    token: String,
}

impl GithubClient {
    fn from_env() -> Result<Self> {
        let token = env::var("GITHUB_TOKEN")
            .or_else(|_| env::var("GH_TOKEN"))
            .context("neither GITHUB_TOKEN nor GH_TOKEN is set")?;
        let http = Client::builder()
            .timeout(Duration::from_secs(45))
            .user_agent("rust-review-gate/0.1")
            .build()?;
        Ok(Self { http, token })
    }

    async fn pull_diff(&self, repo: &str, pull_number: u64) -> Result<String> {
        let response = self
            .http
            .get(format!(
                "https://api.github.com/repos/{repo}/pulls/{pull_number}"
            ))
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/vnd.github.v3.diff")
            .header("X-GitHub-Api-Version", "2026-03-10")
            .send()
            .await?;
        self.text_response(response, "fetch pull-request diff")
            .await
    }

    async fn pull_metadata(&self, repo: &str, pull_number: u64) -> Result<PullMetadata> {
        let response = self
            .http
            .get(format!(
                "https://api.github.com/repos/{repo}/pulls/{pull_number}"
            ))
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2026-03-10")
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        ensure_success(status, &body, "fetch pull-request metadata")?;
        let pull: PullResponse =
            serde_json::from_str(&body).context("decode pull-request metadata")?;
        Ok(PullMetadata {
            head_sha: pull.head.sha,
            title: pull.title,
            body: pull.body.unwrap_or_default(),
        })
    }

    async fn pull_head_sha(&self, repo: &str, pull_number: u64) -> Result<String> {
        Ok(self.pull_metadata(repo, pull_number).await?.head_sha)
    }

    async fn post_review(&self, repo: &str, pull_number: u64, payload: &Value) -> Result<()> {
        let response = self
            .http
            .post(format!(
                "https://api.github.com/repos/{repo}/pulls/{pull_number}/reviews"
            ))
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2026-03-10")
            .json(payload)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        ensure_success(status, &body, "post pull-request review")
    }

    async fn text_response(&self, response: reqwest::Response, operation: &str) -> Result<String> {
        let status = response.status();
        let body = response.text().await?;
        ensure_success(status, &body, operation)?;
        Ok(body)
    }
}

fn ensure_success(status: StatusCode, body: &str, operation: &str) -> Result<()> {
    if status.is_success() {
        return Ok(());
    }
    let excerpt: String = body.chars().take(1_000).collect();
    Err(anyhow!("{operation} failed with {status}: {excerpt}"))
}

#[cfg(test)]
mod tests {
    use super::validate_repo;

    #[test]
    fn validates_repo_slug() {
        assert!(validate_repo("codeandsolder/rust-review-gate").is_ok());
        assert!(validate_repo("codeandsolder/../secret").is_err());
        assert!(validate_repo("missing-owner").is_err());
    }
}
