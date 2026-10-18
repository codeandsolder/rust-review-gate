use std::collections::BTreeSet;

use serde::Serialize;

use crate::model::{Finding, ModelRun};

#[derive(Debug, Clone, Serialize)]
pub struct ConsensusFinding {
    pub file: String,
    pub line: u32,
    pub rule: String,
    pub votes: usize,
    pub voters: Vec<String>,
    pub severity: String,
    pub confidence: f32,
    pub title: String,
    pub explanation: String,
    pub evidence: String,
    pub suggested_test: Option<String>,
}

struct Cluster {
    file: String,
    line: u32,
    rule: String,
    voters: BTreeSet<String>,
    representative: Finding,
}

#[must_use]
pub fn consensus(
    runs: &[ModelRun],
    min_votes: usize,
    line_tolerance: u32,
) -> Vec<ConsensusFinding> {
    let mut clusters: Vec<Cluster> = Vec::new();

    for run in runs {
        for finding in &run.review.findings {
            for rule in &finding.rule_refs {
                let existing = clusters.iter_mut().find(|cluster| {
                    cluster.file == finding.file
                        && cluster.rule == *rule
                        && cluster.line.abs_diff(finding.line) <= line_tolerance
                });

                if let Some(cluster) = existing {
                    cluster.voters.insert(run.model.clone());
                    if stronger(finding, &cluster.representative) {
                        cluster.line = finding.line;
                        cluster.representative = finding.clone();
                    }
                } else {
                    clusters.push(Cluster {
                        file: finding.file.clone(),
                        line: finding.line,
                        rule: rule.clone(),
                        voters: BTreeSet::from([run.model.clone()]),
                        representative: finding.clone(),
                    });
                }
            }
        }
    }

    let mut accepted: Vec<_> = clusters
        .into_iter()
        .filter(|cluster| cluster.voters.len() >= min_votes)
        .map(|cluster| ConsensusFinding {
            file: cluster.file,
            line: cluster.line,
            rule: cluster.rule,
            votes: cluster.voters.len(),
            voters: cluster.voters.into_iter().collect(),
            severity: cluster.representative.severity,
            confidence: cluster.representative.confidence,
            title: cluster.representative.title,
            explanation: cluster.representative.explanation,
            evidence: cluster.representative.evidence,
            suggested_test: cluster.representative.suggested_test,
        })
        .collect();

    accepted.sort_by(|a, b| (&a.file, a.line, &a.rule).cmp(&(&b.file, b.line, &b.rule)));
    accepted
}

fn stronger(candidate: &Finding, current: &Finding) -> bool {
    severity_rank(&candidate.severity)
        .cmp(&severity_rank(&current.severity))
        .then_with(|| candidate.confidence.total_cmp(&current.confidence))
        .is_gt()
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "block" => 3,
        "major" => 2,
        "minor" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::consensus;
    use crate::model::{Finding, ModelRun, Review};

    fn run(model: &str, line: u32, rule: &str) -> ModelRun {
        ModelRun {
            model: model.to_owned(),
            review: Review {
                verdict: "comment".to_owned(),
                findings: vec![Finding {
                    severity: "major".to_owned(),
                    confidence: 0.9,
                    file: "src/lib.rs".to_owned(),
                    line,
                    title: "problem".to_owned(),
                    explanation: "breaks under a concrete condition".to_owned(),
                    evidence: "changed branch".to_owned(),
                    rule_refs: vec![rule.to_owned()],
                    suggested_test: None,
                }],
            },
            prompt_tokens: None,
            completion_tokens: None,
            cost: None,
            elapsed_ms: 1,
        }
    }

    #[test]
    fn requires_distinct_model_agreement() {
        let runs = vec![
            run("a", 10, "displayed-error"),
            run("b", 12, "displayed-error"),
            run("c", 40, "api-contract"),
        ];
        let accepted = consensus(&runs, 2, 3);
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].rule, "displayed-error");
        assert_eq!(accepted[0].votes, 2);
    }
}
