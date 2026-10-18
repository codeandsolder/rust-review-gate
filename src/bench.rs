use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    diff::ParsedDiff,
    model::{Finding, ModelClient},
    review::{build_prompt, scan_added_lines},
};

#[derive(Debug, Deserialize)]
struct GroundTruth {
    name: String,
    diff: String,
    expected_rules: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FixtureScore {
    fixture: String,
    expected_rules: Vec<String>,
    found_rules: Vec<String>,
    true_positive_rules: usize,
    missed_rules: usize,
    false_positive_rules: usize,
    verdict: String,
    findings: usize,
    anchored_findings: usize,
    findings_detail: Vec<Finding>,
    cost: Option<f64>,
    elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
struct ModelScore {
    model: String,
    fixtures: Vec<FixtureScore>,
    rule_recall_ppm: u32,
    precision_ppm: u32,
    clean_false_positive_findings: usize,
    total_findings: usize,
    total_cost: f64,
    errors: Vec<String>,
}

pub async fn run(
    fixtures_dir: &Path,
    models: &[String],
    temperature: f32,
    output: Option<&Path>,
) -> Result<()> {
    let fixtures = load_fixtures(fixtures_dir)?;
    let client = ModelClient::from_env()?;
    let mut model_scores = Vec::new();

    for model in models {
        eprintln!("benchmarking {model}");
        let mut scores = Vec::new();
        let mut errors = Vec::new();

        for fixture in &fixtures {
            match run_fixture(&client, model, fixture, temperature).await {
                Ok(score) => scores.push(score),
                Err(error) => errors.push(format!("{}: {error:#}", fixture.name)),
            }
        }

        let expected = scores
            .iter()
            .map(|score| score.expected_rules.len())
            .sum::<usize>();
        let tp = scores
            .iter()
            .map(|score| score.true_positive_rules)
            .sum::<usize>();
        let fp = scores
            .iter()
            .map(|score| score.false_positive_rules)
            .sum::<usize>();
        let total_findings = scores.iter().map(|score| score.findings).sum();
        let clean_false_positive_findings = scores
            .iter()
            .filter(|score| score.expected_rules.is_empty())
            .map(|score| score.findings)
            .sum();
        let total_cost = scores.iter().filter_map(|score| score.cost).sum();

        model_scores.push(ModelScore {
            model: model.clone(),
            fixtures: scores,
            rule_recall_ppm: ratio_ppm(tp, expected),
            precision_ppm: ratio_ppm(tp, tp + fp),
            clean_false_positive_findings,
            total_findings,
            total_cost,
            errors,
        });
    }

    let json = serde_json::to_string_pretty(&model_scores)?;
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create output dir {}", parent.display()))?;
        }
        fs::write(path, &json).with_context(|| format!("write {}", path.display()))?;
    }
    println!("{json}");
    Ok(())
}

async fn run_fixture(
    client: &ModelClient,
    model: &str,
    fixture: &GroundTruth,
    temperature: f32,
) -> Result<FixtureScore> {
    let diff_path = PathBuf::from(&fixture.diff);
    let text = fs::read_to_string(&diff_path)
        .with_context(|| format!("read fixture diff {}", diff_path.display()))?;
    let parsed = ParsedDiff::parse(&text);
    let prompt = build_prompt(&parsed, &scan_added_lines(&parsed.added_lines))?;
    let run = client.review(model, &prompt, temperature).await?;

    let found_rules: BTreeSet<String> = run
        .review
        .findings
        .iter()
        .flat_map(|finding| finding.rule_refs.iter().cloned())
        .collect();
    let expected: BTreeSet<String> = fixture.expected_rules.iter().cloned().collect();

    let true_positive_rules = found_rules.intersection(&expected).count();
    let missed_rules = expected.difference(&found_rules).count();
    let false_positive_rules = found_rules.difference(&expected).count();

    let anchored_findings = run
        .review
        .findings
        .iter()
        .filter(|finding| {
            parsed
                .added_lines
                .iter()
                .any(|line| line.file == finding.file && line.line == finding.line)
        })
        .count();
    let findings = run.review.findings;

    Ok(FixtureScore {
        fixture: fixture.name.clone(),
        expected_rules: expected.into_iter().collect(),
        found_rules: found_rules.into_iter().collect(),
        true_positive_rules,
        missed_rules,
        false_positive_rules,
        verdict: run.review.verdict,
        findings: findings.len(),
        anchored_findings,
        findings_detail: findings,
        cost: run.cost,
        elapsed_ms: run.elapsed_ms,
    })
}

fn load_fixtures(dir: &Path) -> Result<Vec<GroundTruth>> {
    let manifest_path = dir.join("manifest.json");
    let manifest = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let mut fixtures: Vec<GroundTruth> = serde_json::from_str(&manifest)?;
    for fixture in &mut fixtures {
        fixture.diff = dir.join(&fixture.diff).to_string_lossy().into_owned();
    }
    Ok(fixtures)
}

fn ratio_ppm(numerator: usize, denominator: usize) -> u32 {
    if denominator == 0 {
        return 1_000_000;
    }
    let scaled = numerator.saturating_mul(1_000_000) / denominator;
    u32::try_from(scaled).unwrap_or(1_000_000)
}
