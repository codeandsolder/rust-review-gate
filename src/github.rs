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
) -> Result<()> {
    validate_repo(repo)?;
    if min_votes == 0 || min_votes > models.len() {
        return Err(anyhow!(
            "min-votes must be between 1 and the model count ({})",
            models.len()
        ));
    }

    let github = GithubClient::from_env()?;
    let head_sha = github.pull_head_sha(repo, pull_number).await?;
    let diff = github.pull_diff(repo, pull_number).await?;
    let parsed = ParsedDiff::parse(&diff);
    let output = review::review_text(&diff, models, temperature, min_votes).await?;
    if output.runs.len() < min_votes {
        let failed = output
            .failures
            .iter()
            .map(|failure| failure.model.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(anyhow!(
            "only {} reviewer(s) succeeded, below min-votes {min_votes}; failed models: {failed}",
            output.runs.len()
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
    let mut body = format!(
        "rust-review-gate: {} consensus finding(s) from {} successful reviewer(s); {} blocking/major.",
        output.consensus.len(),
        output.runs.len(),
        blocking
    );
    if !output.failures.is_empty() {
        body.push_str("\n\nReviewer failures (excluded from consensus):\n");
        for failure in &output.failures {
            writeln!(
                &mut body,
                "- `{}` unavailable; excluded from consensus",
                failure.model
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

fn format_finding(finding: &crate::aggregate::ConsensusFinding) -> String {
    let mut body = format!(
        "**{}** (`{}`, {} votes)\n\n{}\n\nEvidence: {}",
        finding.title, finding.rule, finding.votes, finding.explanation, finding.evidence
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

    async fn pull_head_sha(&self, repo: &str, pull_number: u64) -> Result<String> {
        #[derive(Deserialize)]
        struct Pull {
            head: Head,
        }
        #[derive(Deserialize)]
        struct Head {
            sha: String,
        }

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
        let pull: Pull = serde_json::from_str(&body).context("decode pull-request metadata")?;
        Ok(pull.head.sha)
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
